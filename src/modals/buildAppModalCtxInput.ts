import { buildAppModalsSource } from './buildAppModalsSource';
import { modalCtxFileFields } from './buildAppModalCtxFileFields';
import { modalCtxPageFields } from './buildAppModalCtxPageFields';
import { modalCtxSecurityFields } from './buildAppModalCtxSecurityFields';
import { modalCtxAnnotFields } from './buildAppModalCtxAnnotFields';
import { modalCtxChromeFields } from './buildAppModalCtxChromeFields';
import type { BuildAppModalCtxInputArgs } from './buildAppModalCtxArgs';

export function buildAppModalCtxInput(args: BuildAppModalCtxInputArgs) {
  return buildAppModalsSource({
    ...modalCtxFileFields(args),
    ...modalCtxPageFields(args),
    ...modalCtxSecurityFields(args),
    ...modalCtxAnnotFields(args),
    ...modalCtxChromeFields(args),
  });
}
