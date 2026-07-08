import type { AppMenuContext, AppMenus, MenuAction, MenuEntry } from './types';
import { flattenMenuActions } from './menuBuilders';
import { buildFileEditMenus } from './buildAppMenusFileEdit';
import { buildPagesMenu } from './buildAppMenusPages';
import { buildDocumentMenu } from './buildAppMenusDocument';
import { buildAnnotMenu, buildSecurityMenu } from './buildAppMenusAnnot';
import { buildHelpMenu, buildQuickAccessActions, buildViewMenu } from './buildAppMenusChrome';
import { shortcutToDisplay } from '../settings/shortcutKeys';


function resolveMenuShortcuts(entries: MenuEntry[], bindings: AppMenuContext['shortcutBindings']): void {
  for (const entry of entries) {
    if ('separator' in entry) continue;
    if ('items' in entry && !('id' in entry)) {
      resolveMenuShortcuts(entry.items, bindings);
      continue;
    }
    const action = entry as MenuAction;
    if (action.shortcutCommandId) {
      const shortcuts = bindings[action.shortcutCommandId];
      action.shortcut = shortcuts?.length ? shortcutToDisplay(shortcuts[0]) : undefined;
    }
  }
}

export function buildAppMenus(ctx: AppMenuContext): AppMenus {
  const { fileMenu, editMenu } = buildFileEditMenus(ctx);
  const pagesMenu = buildPagesMenu(ctx);
  const documentMenu = buildDocumentMenu(ctx);
  const annotateMenu = buildAnnotMenu(ctx);
  const securityMenu = buildSecurityMenu(ctx);
  const viewMenu = buildViewMenu(ctx);
  const helpMenu = buildHelpMenu(ctx);

  const pdfRequiredMenuIds = new Set(['edit', 'pages', 'document', 'annotate', 'security']);
  const menus = [fileMenu, editMenu, pagesMenu, documentMenu, annotateMenu, securityMenu, viewMenu, helpMenu].filter(
    (menu) => menu.items.length > 0 && (ctx.hasPdf || !pdfRequiredMenuIds.has(menu.id)),
  );
  for (const menu of menus) {
    resolveMenuShortcuts(menu.items, ctx.shortcutBindings);
  }

  const quickAccess = buildQuickAccessActions(ctx);
  resolveMenuShortcuts(quickAccess, ctx.shortcutBindings);

  const allActions = flattenMenuActions(menus);

  return { menus, quickAccess, allActions };
}
