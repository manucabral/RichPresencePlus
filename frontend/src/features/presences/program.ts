/** A readable presence name from an executable: `my-editor.exe` is `My Editor`. */
export function displayName(program: string): string {
  const words = program.replace(/\.exe$/i, "").split(/[-_.\s]+/).filter(Boolean);
  return words.map((word) => word.charAt(0).toUpperCase() + word.slice(1)).join(" ");
}
