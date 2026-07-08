type PrintSurfaceProps = {
  pages: string[];
};

export function PrintSurface({ pages }: PrintSurfaceProps) {
  return (
    <div className="print-root">
      {pages.map((src, i) => (
        <img key={i} src={src} className="print-page" alt={`Print page ${i + 1}`} />
      ))}
    </div>
  );
}
