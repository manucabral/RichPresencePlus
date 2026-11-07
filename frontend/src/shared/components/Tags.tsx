import { useT } from "../i18n";

interface Props {
  author: string | null;
  tags: string[];
}

/** Author and tags in one muted line of chips. */
export default function Tags({ author, tags }: Props) {
  const { t } = useT();
  if (!author && tags.length === 0) return null;
  return (
    <div className="mt-1 flex flex-wrap items-center gap-1.5 text-[11px] text-neutral-500">
      {author && <span>{author}</span>}
      {tags.map((tag) => (
        <span key={tag} className="px-1.5 rounded bg-neutral-800/70 text-neutral-400">
          {t(`tag.${tag}`)}
        </span>
      ))}
    </div>
  );
}
