declare module '*.css' {
  const content: Record<string, string>;
  export default content;
}

declare module '*.png' {
  const src: string;
  export default src;
}

interface ImportMetaEnv {
  readonly VITE_WDIO?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
