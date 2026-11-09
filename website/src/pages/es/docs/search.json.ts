import { searchIndex } from "../../../data/docs-index";

/** The docs search reads this only when someone starts typing. */
export function GET() {
  return new Response(JSON.stringify(searchIndex("es")), { headers: { "content-type": "application/json" } });
}
