import type { ComponentChildren } from "preact";

interface Props {
  title: ComponentChildren;
  /** Starts open. */
  open?: boolean;
  className?: string;
  /** Lets rows reach the edges, with no padding around them. */
  flush?: boolean;
  children: ComponentChildren;
}

/** A section that opens and closes; its styles are in design/accordion.css. */
export default function Accordion({ title, open = false, className = "", flush = false, children }: Props) {
  return (
    <div className={`accordion ${className}`}>
      <details open={open}>
        <summary>{title}</summary>
        <div className={`accordion-body ${flush ? "accordion-flush" : ""}`}>{children}</div>
      </details>
    </div>
  );
}
