import { forwardRef, type ReactNode } from 'react';

type SettingsCardProps = {
  title: string;
  subtitle?: string;
  children: ReactNode;
  id?: string;
  tabIndex?: number;
};

export const SettingsCard = forwardRef<HTMLElement, SettingsCardProps>(function SettingsCard(
  { title, subtitle, children, id, tabIndex },
  ref,
) {
  return (
    <section ref={ref} id={id} tabIndex={tabIndex} className="settings-card">
      <div className="settings-card-header">
        <h2 className="settings-card-title">{title}</h2>
        {subtitle && <p className="settings-card-subtitle">{subtitle}</p>}
      </div>
      <div className="settings-card-body">{children}</div>
    </section>
  );
});
