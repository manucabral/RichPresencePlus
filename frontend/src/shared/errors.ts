export function errorMessage(error: unknown): string {
  const text = error instanceof Error ? error.message : String(error);
  return text.replace(/^\w*Error:\s*/, "");
}
