import type { JSX } from "preact";
import type { ComponentChildren } from "preact";

type Variant = "primary" | "danger" | "neutral" | "ghost" | "warning";

type Props = Omit<JSX.IntrinsicElements["button"], "children"> & {
  children: ComponentChildren;
  variant?: Variant;
  loading?: boolean;
};

const base =
  "px-4 py-2 text-sm font-medium rounded-lg focus:outline-none inline-flex items-center justify-center gap-2 whitespace-nowrap";

const variantClasses: Record<Variant, string> = {
  primary: "bg-green-500/10 text-green-400 hover:bg-green-500/20",
  danger: "bg-red-500/10 text-red-400 hover:bg-red-500/20",
  neutral: "bg-neutral-800 text-neutral-400 hover:bg-neutral-700",
  ghost: "bg-transparent text-neutral-200 hover:bg-neutral-800/40",
  warning: "bg-amber-500/10 text-amber-400 hover:bg-amber-500/20",
};

export default function Button({
  children,
  variant = "neutral",
  loading = false,
  className = "",
  disabled,
  ...rest
}: Props) {
  const isDisabled = disabled || loading;

  const classes = `${base} ${variantClasses[variant]} ${
    isDisabled ? "opacity-60 cursor-not-allowed" : ""
  } ${className}`.trim();

  return (
    <button {...(rest as any)} className={classes} disabled={isDisabled}>
      <span className="flex items-center gap-2 whitespace-nowrap">
        {loading && (
          <svg
            className="animate-spin h-4 w-4 text-current"
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
          >
            <circle
              className="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              strokeWidth="4"
            ></circle>
            <path
              className="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8v4a4 4 0 00-4 4H4z"
            ></path>
          </svg>
        )}
        <span className="flex items-center gap-2">{children}</span>
      </span>
    </button>
  );
}
