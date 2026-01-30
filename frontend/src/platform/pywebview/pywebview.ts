type PyApi = NonNullable<Window["pywebview"]>["api"];

export function getPyApi(): Promise<PyApi> {
  return new Promise((resolve) => {
    if (window.pywebview?.api) {
      // already inyected
      resolve(window.pywebview.api);
      return;
    }
    window.addEventListener(
      // fallback
      "pywebviewready",
      () => resolve(window.pywebview!.api),
      { once: true },
    );
  });
}
