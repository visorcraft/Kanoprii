import { buildAppMenuSource, type BuildAppMenuSourceInput } from './buildAppMenuSource';
import { buildAppMenus } from './buildAppMenus';
import { menuContextDocAnnotFields } from './buildAppMenuContextDocAnnotFields';
import { menuContextPagesFields } from './buildAppMenuContextPagesFields';
import type { BuildAppMenuInputArgs } from './buildAppMenuInputArgs';
import { menuInputDocFields } from './buildAppMenuInputDocFields';
import { menuInputPagesFields } from './buildAppMenuInputPagesFields';

export function buildAppMenuInput(args: BuildAppMenuInputArgs) {
  const source = buildAppMenuSource({
    ...menuInputDocFields(args),
    ...menuInputPagesFields(args),
    surface: args.surface,
    workspace: args.workspace,
    shortcutBindings: args.shortcutBindings,
  } satisfies BuildAppMenuSourceInput);

  return buildAppMenus({
    ...menuContextDocAnnotFields(source),
    ...menuContextPagesFields(source),
    activeSurface: source.activeSurface,
    openSettings: source.openSettings,
    shortcutBindings: source.shortcutBindings,
  });
}
