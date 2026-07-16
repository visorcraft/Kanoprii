import type { ReactNode } from 'react';
import type { MenuAction, MenuEntry, MenuRoot, MenuSubmenu } from '../menu/types';
import { sep } from '../menu/menuBuilders';
import type { RibbonIconName } from './icons';

export type RibbonButton = { kind: 'button'; action: MenuAction; icon: RibbonIconName };
export type RibbonDropdownDef = {
  kind: 'dropdown';
  id: string;
  label: string;
  icon: RibbonIconName;
  items: MenuEntry[];
  danger?: boolean;
};
export type RibbonControl = RibbonButton | RibbonDropdownDef;

export type RibbonGroup = { id: string; label: string; controls: RibbonControl[] };

export type RibbonTabDef =
  | { kind: 'menu'; id: string; label: string; disabled?: boolean; items: MenuEntry[] }
  | { kind: 'tab'; id: string; label: string; disabled?: boolean; groups: RibbonGroup[]; extras?: ReactNode };

const btn = (root: MenuRoot, id: string, icon: RibbonIconName): RibbonButton => ({
  kind: 'button',
  action: actionById(root, id),
  icon,
});

const dd = (
  root: MenuRoot,
  id: string,
  label: string,
  icon: RibbonIconName,
  danger = false,
  submenuLabel = label,
): RibbonDropdownDef => ({
  kind: 'dropdown',
  id,
  label,
  icon,
  items: submenuByLabel(root, submenuLabel).items,
  danger,
});

function actionById(root: MenuRoot, id: string): MenuAction {
  const entry = root.items.find((e) => 'id' in e && e.id === id);
  if (!entry) throw new Error(`ribbonModel: action "${id}" missing in menu "${root.id}"`);
  return entry as MenuAction;
}

function submenuByLabel(root: MenuRoot, label: string): MenuSubmenu {
  const entry = root.items.find((e) => !('id' in e) && 'items' in e && e.label === label);
  if (!entry) throw new Error(`ribbonModel: submenu "${label}" missing in menu "${root.id}"`);
  return entry as MenuSubmenu;
}

export function buildRibbonTabs(input: {
  menus: MenuRoot[];
  quickAccess: MenuAction[];
  editTabContent: ReactNode;
  annotateOptions: ReactNode;
}): RibbonTabDef[] {
  const byId = new Map(input.menus.map((m) => [m.id, m]));
  const file = byId.get('file');
  const edit = byId.get('edit');
  const pages = byId.get('pages');
  const doc = byId.get('document');
  const annot = byId.get('annotate');
  const security = byId.get('security');
  const view = byId.get('view');
  const help = byId.get('help');

  const qa = (id: string): MenuAction => {
    const found = input.quickAccess.find((a) => a.id === id);
    if (!found) throw new Error(`ribbonModel: quick action "${id}" missing`);
    return found;
  };

  const tabs: RibbonTabDef[] = [];

  tabs.push({ kind: 'menu', id: 'file', label: 'File', items: file?.items ?? [] });

  tabs.push({
    kind: 'tab',
    id: 'home',
    label: 'Home',
    groups: [
      { id: 'home-file', label: 'File', controls: [ { kind: 'button', action: qa('qa-save'), icon: 'save' } ] },
      { id: 'home-history', label: 'History', controls: [
        { kind: 'button', action: qa('qa-undo'), icon: 'undo' },
        { kind: 'button', action: qa('qa-redo'), icon: 'redo' },
      ] },
      { id: 'home-tools', label: 'Tools', controls: [
        { kind: 'button', action: qa('qa-select'), icon: 'select' },
        { kind: 'button', action: qa('qa-find'), icon: 'find' },
      ] },
      { id: 'home-page', label: 'Page', controls: [
        { kind: 'button', action: qa('qa-rotate'), icon: 'rotate' },
        { kind: 'button', action: qa('qa-dup'), icon: 'duplicate' },
      ] },
    ],
  });

  tabs.push({
    kind: 'tab',
    id: 'annotate',
    label: 'Annotate',
    disabled: !annot,
    groups: annot ? [
      { id: 'annot-markup', label: 'Markup', controls: [
        btn(annot, 'highlight', 'highlight'),
        btn(annot, 'highlight-selection', 'highlight'),
        btn(annot, 'note', 'note'),
        btn(annot, 'draw', 'draw'),
      ] },
      { id: 'annot-insert', label: 'Insert', controls: [
        btn(annot, 'shape', 'shape'),
        btn(annot, 'stamp', 'stamp'),
      ] },
      { id: 'annot-redact', label: 'Redact', controls: [
        btn(annot, 'redact', 'redact'),
      ] },
    ] : [],
    extras: input.annotateOptions ?? undefined,
  });

  tabs.push({
    kind: 'tab',
    id: 'edit',
    label: 'Edit',
    disabled: !edit,
    groups: [],
    extras: input.editTabContent ?? undefined,
  });

  tabs.push({
    kind: 'tab',
    id: 'pages',
    label: 'Pages',
    disabled: !pages,
    groups: pages ? [
      { id: 'pages-organize', label: 'Organize', controls: [
        btn(pages, 'rot-modal', 'rotate'),
        dd(pages, 'rotate', 'Rotate', 'rotate'),
        dd(pages, 'duplicate', 'Duplicate', 'duplicate'),
        dd(pages, 'move-order', 'Move & order', 'move'),
      ] },
      { id: 'pages-insert-delete', label: 'Insert & delete', controls: [
        dd(pages, 'insert', 'Insert', 'insert'),
        dd(pages, 'delete', 'Delete', 'delete', true),
      ] },
      { id: 'pages-extract', label: 'Extract & combine', controls: [
        dd(pages, 'split-extract', 'Split & extract', 'split'),
        dd(pages, 'combine', 'Combine', 'combine'),
        dd(pages, 'keep-filter', 'Keep & filter', 'keep'),
      ] },
      { id: 'pages-sort', label: 'Sort & parity', controls: [
        dd(pages, 'sort', 'Sort', 'sort'),
        btn(pages, 'parity-range', 'parity'),
      ] },
    ] : [],
  });

  tabs.push({
    kind: 'tab',
    id: 'document',
    label: 'Document',
    disabled: !doc,
    groups: doc ? [
      { id: 'doc-process', label: 'Process', controls: [
        btn(doc, 'optimize', 'optimize'),
        btn(doc, 'make-searchable', 'ocr'),
        btn(doc, 'apply-redactions', 'redactions'),
        btn(doc, 'summarize', 'summarize'),
      ] },
      { id: 'doc-layout', label: 'Layout', controls: [
        btn(doc, 'page-numbers', 'page-numbers'),
        btn(doc, 'bates-numbers', 'bates'),
        btn(doc, 'page-header', 'header'),
        btn(doc, 'page-footer', 'footer'),
        btn(doc, 'page-size', 'page-size'),
        btn(doc, 'watermark', 'watermark'),
        btn(doc, 'border', 'border'),
        {
          kind: 'dropdown',
          id: 'crop-margins',
          label: 'Crop & margins',
          icon: 'crop',
          items: [
            submenuByLabel(doc, 'Crop'),
            sep(),
            actionById(doc, 'expand'),
            actionById(doc, 'shrink'),
          ],
        },
        dd(doc, 'flatten', 'Flatten', 'flatten', false, 'Flatten annotations'),
      ] },
      { id: 'doc-info', label: 'Info', controls: [
        btn(doc, 'metadata', 'metadata'),
      ] },
      { id: 'doc-sign', label: 'Sign', controls: security ? [
        btn(security, 'sign', 'sign'),
        btn(security, 'signatures', 'panel'),
      ] : [] },
    ] : [],
  });

  tabs.push({
    kind: 'tab',
    id: 'view',
    label: 'View',
    disabled: !view,
    groups: view ? [
      { id: 'view-workspace', label: 'Workspace', controls: [
        btn(view, 'view-pdf', 'view-pdf'),
        btn(view, 'view-birdseye', 'view-birdseye'),
        btn(view, 'view-md', 'view-markdown'),
      ] },
      { id: 'view-panels', label: 'Panels', controls: [
        btn(view, 'thumbnails', 'panel'),
        btn(view, 'bookmarks', 'panel'),
        btn(view, 'annotations-panel', 'panel'),
        btn(view, 'forms', 'panel'),
        ...(security ? [btn(security, 'signatures', 'panel')] : []),
      ] },
      { id: 'view-display', label: 'Display', controls: [
        btn(view, 'continuous-scroll', 'continuous'),
        btn(view, 'show-hidden-layers', 'hidden-layers'),
        btn(view, 'pdfua-panel', 'pdfua'),
      ] },
    ] : [],
  });

  tabs.push({ kind: 'menu', id: 'help', label: 'Help', items: help?.items ?? [] });

  return tabs;
}
