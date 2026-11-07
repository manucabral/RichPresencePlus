import { Component, type ComponentChildren } from "preact";
import { t } from "../i18n";
import Button from "./Button";

interface Props {
  children: ComponentChildren;
}

interface State {
  failed: boolean;
}

/**
 * Keeps a page that breaks from taking the whole window with it: the menu stays
 * and the page says what to do. It sits around the page, keyed by the address,
 * so going to another page starts clean.
 */
export default class PageGuard extends Component<Props, State> {
  state: State = { failed: false };

  static getDerivedStateFromError(): State {
    return { failed: true };
  }

  override componentDidCatch(error: unknown) {
    console.error(error);
  }

  override render() {
    if (!this.state.failed) return this.props.children;
    return (
      <div role="alert" className="mx-auto w-full max-w-3xl pt-2">
        <p className="text-sm font-medium text-neutral-100">{t("page.failed")}</p>
        <p className="mt-1 text-[13px] text-neutral-500">{t("page.failedHint")}</p>
        {/* A page that did not load stays failed until the window starts again. */}
        <Button variant="primary" className="mt-3" onClick={() => location.reload()}>
          {t("page.retry")}
        </Button>
      </div>
    );
  }
}
