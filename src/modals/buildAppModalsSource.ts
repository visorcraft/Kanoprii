import { buildAppModalsContext, type AppModalsRuntime } from './appModalsContext';

/** Assemble modal wiring context from App hook/state outputs. */
export function buildAppModalsSource(ctx: AppModalsRuntime): AppModalsRuntime {
  return buildAppModalsContext(ctx);
}
