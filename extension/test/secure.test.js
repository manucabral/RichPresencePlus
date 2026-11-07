import assert from "node:assert/strict";
import { test } from "node:test";
import { checkProof, createSession, fromBase64, newNonce, proof, toBase64 } from "../secure.js";

// The same values are used by the tests of crates/rpp-bridge/src/secure.rs,
// so these vectors prove both halves compute the same bytes.
const TOKEN = "7c13347b8b6a4e8e9863498d5c534a5651cf8ee4565a419e95007e64b42b2bbc";
const SERVER = toBase64(new Uint8Array(32).fill(1));
const CLIENT = toBase64(new Uint8Array(32).fill(2));

test("the hello proof matches the application's", async () => {
  assert.equal(await proof(TOKEN, "rpp-hello", SERVER, CLIENT), "FZysmTHImAUhRay3jFgUrcBKDODjQ+kz9W/gyQTksHY=");
});

test("a message sealed by the application opens here", async () => {
  const session = await createSession(TOKEN, SERVER, CLIENT);
  const text = await session.open({ type: "sealed", n: 0, data: "0Fc5O/u3PEiDXU9ZEfkxsZbVM33OUSi4dnXfCLzGpg==" });
  assert.equal(text, '{"type":"pong"}');
});

test("a welcome from a program without the token is rejected", async () => {
  const genuine = await proof(TOKEN, "rpp-welcome", SERVER, CLIENT);
  assert.ok(await checkProof(TOKEN, "rpp-welcome", SERVER, CLIENT, genuine));
  const forged = await proof("guessed", "rpp-welcome", SERVER, CLIENT);
  assert.ok(!(await checkProof(TOKEN, "rpp-welcome", SERVER, CLIENT, forged)));
  // The hello proof cannot be sent back as a welcome.
  const hello = await proof(TOKEN, "rpp-hello", SERVER, CLIENT);
  assert.ok(!(await checkProof(TOKEN, "rpp-welcome", SERVER, CLIENT, hello)));
  assert.ok(!(await checkProof(TOKEN, "rpp-welcome", SERVER, CLIENT, "not base64!")));
});

test("sealed messages hide their text, are numbered and refuse replays", async () => {
  const session = await createSession(TOKEN, SERVER, CLIENT);
  const first = JSON.parse(await session.seal('{"type":"ping"}'));
  const second = JSON.parse(await session.seal('{"type":"ping"}'));
  assert.equal(first.n, 0);
  assert.equal(second.n, 1);
  assert.notEqual(first.data, second.data);
  assert.ok(!first.data.includes("ping"));

  const app = await createSession(TOKEN, SERVER, CLIENT);
  const vector = { type: "sealed", n: 0, data: "0Fc5O/u3PEiDXU9ZEfkxsZbVM33OUSi4dnXfCLzGpg==" };
  await app.open(vector);
  await assert.rejects(app.open(vector), /out of order/);
  await assert.rejects(app.open({ ...vector, n: 5 }), /out of order/);
  const tampered = fromBase64(vector.data);
  tampered[0] ^= 1;
  const fresh = await createSession(TOKEN, SERVER, CLIENT);
  await assert.rejects(fresh.open({ ...vector, data: toBase64(tampered) }));
});

test("random values are fresh and 32 bytes long", () => {
  const a = newNonce();
  assert.notEqual(a, newNonce());
  assert.equal(fromBase64(a).length, 32);
});
