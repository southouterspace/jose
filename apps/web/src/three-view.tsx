/**
 * The 3D view — the perspective, orbitable viewport showing the **mass** (CONTEXT.md / ADR 0008).
 *
 * Per ADR 0005 this is an **imperative Three.js renderer** mounted into a React container via
 * ref+effect: React owns no per-frame state. The scene (camera, lights, grid, OrbitControls) is
 * built once; a rebuild effect disposes+recreates the mass mesh whenever the canonical volume or
 * footprint mirror changes. The mesh is a **presentation tessellation** of `footprint + height`
 * (ADR 0006/0008 §2) — it holds no geometry of its own.
 *
 * Push/pull: with the Push/Pull tool active, a pointer-down that raycasts the **top cap** starts a
 * drag; vertical pointer movement maps (via `pushPullDistance`) to a signed tick delta dispatched as
 * `PushPull { volumeId, TOP_FACE, distance }`. Orbit is disabled during the drag; otherwise the
 * pointer orbits/zooms the camera.
 */

import type { FootprintMirror, VolumeMirror } from "@jose/render-mirror";
import { pushPullDistance } from "@jose/tool-runner";
import { useEffect, useRef } from "react";
import {
  Color,
  DirectionalLight,
  DoubleSide,
  EdgesGeometry,
  ExtrudeGeometry,
  GridHelper,
  Group,
  HemisphereLight,
  LineBasicMaterial,
  LineSegments,
  Mesh,
  MeshStandardMaterial,
  PerspectiveCamera,
  Raycaster,
  Scene,
  Shape,
  ShapeGeometry,
  Vector2,
  WebGLRenderer,
} from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import type { EngineStore } from "./engine-store";
import { frameMass, TICKS_PER_UNIT } from "./mass-tessellation";

/** The kernel's top cap face index (`crates/geometry-kernel/src/brep.rs` `TOP_FACE`). The 3D view
 *  references the engine's named face — it never invents one (ADR 0008 §3). */
const TOP_FACE = 1;

/** World ticks per screen pixel for the push/pull drag. ~6 ticks/px ≈ a comfortable drag feel. */
const PUSHPULL_TICKS_PER_PIXEL = 6;

/** The imperative scene handle: long-lived objects the effects share via a single ref. */
interface SceneHandle {
  readonly camera: PerspectiveCamera;
  readonly controls: OrbitControls;
  /** Signature of the footprint the camera was last framed against. A *footprint* change re-frames
   *  the view; a *height-only* (push/pull) change must not yank the camera (the user may be mid-
   *  orbit). `null` until the first frame is set. */
  footprintSig: string | null;
  /** The current mass group (base + top + sides), or null before the first volume. */
  massGroup: Group | null;
  readonly renderer: WebGLRenderer;
  readonly scene: Scene;
  /** The pickable top-cap mesh, identified for raycasting; null until a mass exists. */
  topMesh: Mesh | null;
  /** Latest volumeId the mass renders, for push/pull commands. */
  volumeId: number;
}

/** Build the footprint as a Three Shape on the XZ ground plane (extrude then runs +Y). */
function footprintShape(footprint: FootprintMirror): Shape | null {
  const verts = footprint.vertices();
  if (verts.length < 3) {
    return null;
  }
  const shape = new Shape();
  const first = verts[0];
  if (!first) {
    return null;
  }
  // Negate the shape's Y: after `rotateX(-90°)` (below) a shape coord (x, y) lands at world
  // (x, …, -y), so we pre-negate Y here to keep world-Z = +footprint-Y — the same convention
  // `planToThree`/`frameMass` use, so the camera frames the mesh where it actually sits.
  shape.moveTo(first.x / TICKS_PER_UNIT, -first.y / TICKS_PER_UNIT);
  for (let i = 1; i < verts.length; i++) {
    const v = verts[i];
    if (v) {
      shape.lineTo(v.x / TICKS_PER_UNIT, -v.y / TICKS_PER_UNIT);
    }
  }
  shape.closePath();
  return shape;
}

/** A stable signature of a footprint's *plan geometry* (vertex count + coordinates), so a redraw is
 *  distinguishable from a height-only push/pull. Excludes height by construction. */
function footprintSignature(footprint: FootprintMirror): string {
  return footprint
    .vertices()
    .map((v) => `${v.x},${v.y}`)
    .join(";");
}

/** Re-frame the camera on the mass centroid: pivot on the footprint, place the camera at an angled
 *  offset sized to the footprint's bounds (with margin) so the whole mass is comfortably in view. */
function frameView(
  handle: SceneHandle,
  footprint: FootprintMirror,
  heightTicks: number
): void {
  const { target, camera } = frameMass(footprint.vertices(), heightTicks);
  handle.controls.target.set(target.x, target.y, target.z);
  handle.camera.position.set(camera.x, camera.y, camera.z);
  handle.controls.update();
}

/** Dispose a group's geometries/materials and detach it from the scene. */
function disposeMass(handle: SceneHandle): void {
  const group = handle.massGroup;
  if (!group) {
    return;
  }
  handle.scene.remove(group);
  group.traverse((obj) => {
    if (obj instanceof Mesh) {
      obj.geometry.dispose();
      const mat = obj.material;
      if (Array.isArray(mat)) {
        for (const m of mat) {
          m.dispose();
        }
      } else {
        mat.dispose();
      }
    }
  });
  handle.massGroup = null;
  handle.topMesh = null;
}

/**
 * (Re)build the mass mesh from the canonical footprint + height. The walls are an extruded prism
 * (footprint shape extruded +Y by `height`); the top cap is a **separate, named mesh** so push/pull
 * picking can identify it directly without a normal heuristic.
 */
function rebuildMass(
  handle: SceneHandle,
  footprint: FootprintMirror | null,
  volume: VolumeMirror | null
): void {
  disposeMass(handle);
  if (!(footprint && volume) || volume.count < 1) {
    return;
  }
  const shape = footprintShape(footprint);
  const vol = volume.row(0);
  const heightUnits = vol.height / TICKS_PER_UNIT;
  if (!shape || heightUnits <= 0) {
    return;
  }
  handle.volumeId = vol.volumeId;

  const group = new Group();

  // Walls: extrude the footprint shape (in its local XY) then rotate so extrusion runs +Y (up).
  const wallGeom = new ExtrudeGeometry(shape, {
    depth: heightUnits,
    bevelEnabled: false,
  });
  // The shape lives in XY and extrudes +Z; rotate -90° about X so the extrusion runs +Y (up).
  // The shape's Y was pre-negated in footprintShape so world-Z = +footprint-Y after this rotation.
  wallGeom.rotateX(-Math.PI / 2);
  const wallMesh = new Mesh(
    wallGeom,
    new MeshStandardMaterial({
      color: 0x6e_88_c8,
      transparent: true,
      opacity: 0.55,
      side: DoubleSide,
      roughness: 0.85,
    })
  );
  group.add(wallMesh);

  // Top cap: a separate, named mesh at Y=height — the pickable push/pull face.
  const capGeom = new ShapeGeometry(shape);
  capGeom.rotateX(-Math.PI / 2); // lay flat on XZ
  capGeom.translate(0, heightUnits, 0); // lift to the top
  const topMesh = new Mesh(
    capGeom,
    new MeshStandardMaterial({
      color: 0x9e_c1_ff,
      side: DoubleSide,
      roughness: 0.6,
    })
  );
  topMesh.name = "top-cap";
  group.add(topMesh);

  // Wireframe edges for legibility.
  const edges = new LineSegments(
    new EdgesGeometry(wallGeom),
    new LineBasicMaterial({ color: 0xbc_d0_ff })
  );
  group.add(edges);

  handle.scene.add(group);
  handle.massGroup = group;
  handle.topMesh = topMesh;

  // Re-frame the camera ONLY when the footprint geometry changed (a fresh draw) — not on a
  // height-only push/pull, which must leave the camera where the user left it (possibly mid-orbit).
  const sig = footprintSignature(footprint);
  if (sig !== handle.footprintSig) {
    frameView(handle, footprint, vol.height);
    handle.footprintSig = sig;
  }
}

export interface ThreeViewProps {
  readonly store: EngineStore;
}

export function ThreeView({ store }: ThreeViewProps) {
  const mountRef = useRef<HTMLDivElement>(null);
  const handleRef = useRef<SceneHandle | null>(null);
  // The active tool gates the drag; keep a ref so pointer handlers read the live value.
  const toolRef = useRef(store.activeTool);
  toolRef.current = store.activeTool;
  const pushPullRef = useRef(store.pushPull);
  pushPullRef.current = store.pushPull;

  // Mount-once effect: build the imperative scene and the render loop.
  useEffect(() => {
    const mount = mountRef.current;
    if (!mount) {
      return;
    }

    const width = mount.clientWidth || 1;
    const height = mount.clientHeight || 1;

    const renderer = new WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(window.devicePixelRatio);
    renderer.setSize(width, height);
    mount.appendChild(renderer.domElement);

    const scene = new Scene();
    scene.background = new Color(0x23_23_28);

    const camera = new PerspectiveCamera(50, width / height, 0.1, 2000);
    camera.position.set(24, 22, 28); // an angled orbit looking down at the ground
    camera.lookAt(0, 0, 0);

    const hemi = new HemisphereLight(0xff_ff_ff, 0x40_40_48, 1.1);
    scene.add(hemi);
    const dir = new DirectionalLight(0xff_ff_ff, 1.2);
    dir.position.set(18, 30, 12);
    scene.add(dir);

    const grid = new GridHelper(200, 200, 0x3a_3a_44, 0x2c_2c_34);
    scene.add(grid);

    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.target.set(0, 0, 0);

    const handle: SceneHandle = {
      renderer,
      scene,
      camera,
      controls,
      massGroup: null,
      topMesh: null,
      volumeId: 0,
      footprintSig: null,
    };
    handleRef.current = handle;

    let raf = 0;
    const tick = (): void => {
      controls.update();
      renderer.render(scene, camera);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);

    const onResize = (): void => {
      const w = mount.clientWidth || 1;
      const h = mount.clientHeight || 1;
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      renderer.setSize(w, h);
    };
    const observer = new ResizeObserver(onResize);
    observer.observe(mount);

    // --- Push/pull drag state (imperative; never React per-frame state) ---
    const raycaster = new Raycaster();
    const pointer = new Vector2();
    let dragging = false;
    let dragStartY = 0;
    let dragVolumeId = 0;

    const setPointer = (event: PointerEvent): void => {
      const rect = renderer.domElement.getBoundingClientRect();
      pointer.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
      pointer.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;
    };

    const onPointerDown = (event: PointerEvent): void => {
      if (toolRef.current !== "pushpull" || !handle.topMesh) {
        return; // Not in push/pull mode (or no mass) — let OrbitControls handle it.
      }
      setPointer(event);
      raycaster.setFromCamera(pointer, camera);
      const hit = raycaster.intersectObject(handle.topMesh, false)[0];
      // The top cap is its own named mesh, so any hit on it is the top face; still confirm the
      // world normal is vertical (|y| > 0.5), naming the engine's `TOP_FACE` per ADR 0008 §3.
      const worldNormal = hit?.face?.normal
        .clone()
        .transformDirection(handle.topMesh.matrixWorld);
      if (!(worldNormal && Math.abs(worldNormal.y) > 0.5)) {
        return;
      }
      dragging = true;
      dragStartY = event.clientY;
      dragVolumeId = handle.volumeId;
      controls.enabled = false; // freeze orbit while pushing/pulling
      renderer.domElement.setPointerCapture(event.pointerId);
      event.preventDefault();
    };

    const onPointerUp = (event: PointerEvent): void => {
      if (!dragging) {
        return;
      }
      const distance = pushPullDistance(
        event.clientY - dragStartY,
        PUSHPULL_TICKS_PER_PIXEL
      );
      dragging = false;
      controls.enabled = true;
      renderer.domElement.releasePointerCapture(event.pointerId);
      pushPullRef.current(dragVolumeId, TOP_FACE, distance);
    };

    renderer.domElement.addEventListener("pointerdown", onPointerDown);
    renderer.domElement.addEventListener("pointerup", onPointerUp);

    return () => {
      cancelAnimationFrame(raf);
      observer.disconnect();
      renderer.domElement.removeEventListener("pointerdown", onPointerDown);
      renderer.domElement.removeEventListener("pointerup", onPointerUp);
      disposeMass(handle);
      controls.dispose();
      renderer.dispose();
      if (renderer.domElement.parentNode === mount) {
        mount.removeChild(renderer.domElement);
      }
      handleRef.current = null;
    };
  }, []);

  // Rebuild effect: dispose+recreate the mass whenever the canonical mirrors change.
  useEffect(() => {
    const handle = handleRef.current;
    if (!handle) {
      return;
    }
    rebuildMass(handle, store.footprint, store.volume);
  }, [store.footprint, store.volume]);

  return <div className="three" ref={mountRef} />;
}
