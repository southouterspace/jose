/**
 * Backend configuration, read once from the environment.
 *
 * The persistence boundary is deployable to the Bun runtime or Cloudflare Workers; the same
 * config shape drives both. Secrets (the Neon connection string, the R2 credentials) are never
 * hard-coded — they come from the environment the deploy target injects.
 */

/** The resolved runtime configuration for the persistence boundary. */
export interface ApiConfig {
  /** TCP port the Bun server binds (ignored on Workers, which owns the listener). */
  readonly port: number;
  /** Neon/Postgres connection string for the Drizzle snapshot store; `undefined` ⇒ in-memory. */
  readonly databaseUrl: string | undefined;
  /** Cloudflare R2 bucket name for blob storage; `undefined` ⇒ in-memory blob store. */
  readonly r2Bucket: string | undefined;
}

/** Read configuration from `Bun.env` (falling back to `process.env`-style access via Bun). */
export function loadConfig(env: Record<string, string | undefined> = Bun.env): ApiConfig {
  const rawPort = env.PORT;
  const port = rawPort !== undefined ? Number.parseInt(rawPort, 10) : 8787;
  return {
    port: Number.isFinite(port) ? port : 8787,
    databaseUrl: env.DATABASE_URL,
    r2Bucket: env.R2_BUCKET,
  };
}
