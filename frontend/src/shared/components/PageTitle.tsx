interface PageTitleProps {
  title: string;
  subtitle: string;
}

export default function PageTitle({ title, subtitle }: PageTitleProps) {
  return (
    <div className="mb-6">
      <h1 className="text-[22px] font-semibold text-neutral-100 tracking-tight">{title}</h1>
      <p className="text-neutral-500 mt-1 text-[13px]">{subtitle}</p>
    </div>
  );
}
