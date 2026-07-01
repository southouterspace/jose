/**
 * The measurement value box — SketchUp's VCB, as a small controlled presentational input a surface
 * docks to type an exact dimension for the active tool. Which dimension it means comes from the
 * tool's `value` grammar (ADR 0012 §4): the plan docks it for a **length**, the 3D view for a
 * **height**. Pure/controlled — the surface owns parsing and what a submit commits; this component
 * owns only the markup, the Enter-submits / Escape-cancels grammar, and the styling.
 */

import type { KeyboardEvent } from "react";

/** Modifier state carried with a submit — e.g. the plan's "type a length + Shift = lock to axis". */
export interface SubmitModifiers {
  readonly shiftKey: boolean;
}

export interface ValueBoxProps {
  /** Accessible name for the input (the visible label is decorative). */
  readonly ariaLabel: string;
  /** Whether the box accepts input right now (dimmed + disabled when false). */
  readonly disabled?: boolean;
  /** The short field label, e.g. `Length` or `Height`. */
  readonly label: string;
  /** Escape while focused — a surface can clear/bail out of an in-progress entry. */
  readonly onCancel?: () => void;
  readonly onChange: (value: string) => void;
  /** Enter while focused — commit the typed value; `modifiers` carries the held keys at submit. */
  readonly onSubmit: (modifiers: SubmitModifiers) => void;
  readonly placeholder: string;
  readonly value: string;
}

export function ValueBox(props: ValueBoxProps) {
  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>): void => {
    if (event.key === "Enter") {
      event.preventDefault();
      props.onSubmit({ shiftKey: event.shiftKey });
    } else if (event.key === "Escape") {
      event.preventDefault();
      props.onCancel?.();
    }
  };

  const disabled = props.disabled ?? false;
  return (
    <label className="value-box" data-active={!disabled}>
      <span className="value-box__label">{props.label}</span>
      <input
        aria-label={props.ariaLabel}
        className="value-box__input"
        disabled={disabled}
        inputMode="text"
        onChange={(event) => props.onChange(event.target.value)}
        onKeyDown={onKeyDown}
        placeholder={props.placeholder}
        type="text"
        value={props.value}
      />
    </label>
  );
}
