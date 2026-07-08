import type { modalCtxFileFields } from './buildAppModalCtxFileFields';
import type { modalCtxPageFields } from './buildAppModalCtxPageFields';
import type { modalCtxSecurityFields } from './buildAppModalCtxSecurityFields';
import type { modalCtxAnnotFields } from './buildAppModalCtxAnnotFields';
import type { modalCtxChromeFields } from './buildAppModalCtxChromeFields';

/** Modal wiring context - all fields supplied by App.tsx. */
export type AppModalsContext = AppModalsRuntime;

/** Typed view used inside AppModals, derived from the field builders so renames surface at compile time. */
export type AppModalsRuntime = ReturnType<typeof modalCtxFileFields> &
  ReturnType<typeof modalCtxPageFields> &
  ReturnType<typeof modalCtxSecurityFields> &
  ReturnType<typeof modalCtxAnnotFields> &
  ReturnType<typeof modalCtxChromeFields>;

export function buildAppModalsContext(ctx: AppModalsRuntime): AppModalsRuntime {
  return ctx;
}
