// Mutual authentication and encryption of the connection to the desktop app.
// This is the extension's half of `crates/rpp-bridge/src/secure.rs`; the two
// must stay in step. The pairing token never travels on the connection:
//
// 1. The app sends a random challenge.
// 2. The extension answers with its own random value and an HMAC-SHA256 of
//    both, keyed with the token.
// 3. The app answers with its own proof. A program that took the port while
//    the app was closed cannot produce it, so it never receives page data.
// 4. Every later message is sealed with AES-256-GCM, one key per direction,
//    numbered so that a repeated or reordered message is refused.

const NONCE_BYTES = 32;
const encoder = new TextEncoder();
const decoder = new TextDecoder();

/** Bytes as base64. */
export function toBase64(bytes) {
  let text = "";
  for (const byte of bytes) text += String.fromCharCode(byte);
  return btoa(text);
}

/** Base64 as bytes; throws on malformed input. */
export function fromBase64(text) {
  return Uint8Array.from(atob(text), (char) => char.charCodeAt(0));
}

/** A fresh random value, as base64. */
export function newNonce() {
  return toBase64(crypto.getRandomValues(new Uint8Array(NONCE_BYTES)));
}

async function hmacKey(token) {
  return crypto.subtle.importKey("raw", encoder.encode(token), { name: "HMAC", hash: "SHA-256" }, false, [
    "sign",
    "verify",
  ]);
}

function label(name, server, client) {
  return encoder.encode(`${name}|${server}|${client}`);
}

/**
 * The proof one side sends to show it knows the token.
 *
 * @param {string} token The pairing token.
 * @param {string} name `rpp-hello` for the extension, `rpp-welcome` for the app.
 * @param {string} server The app's random value, as base64.
 * @param {string} client The extension's random value, as base64.
 * @returns {Promise<string>} The proof, as base64.
 */
export async function proof(token, name, server, client) {
  const signature = await crypto.subtle.sign("HMAC", await hmacKey(token), label(name, server, client));
  return toBase64(new Uint8Array(signature));
}

/**
 * Checks a proof; the comparison inside WebCrypto takes constant time.
 *
 * @returns {Promise<boolean>} True when it matches.
 */
export async function checkProof(token, name, server, client, given) {
  let bytes;
  try {
    bytes = fromBase64(given);
  } catch {
    return false;
  }
  return crypto.subtle.verify("HMAC", await hmacKey(token), bytes, label(name, server, client));
}

async function directionKey(token, name, server, client) {
  const raw = await crypto.subtle.sign("HMAC", await hmacKey(token), label(name, server, client));
  return crypto.subtle.importKey("raw", raw, "AES-GCM", false, ["encrypt", "decrypt"]);
}

/** The 96-bit nonce of message `n`: four zero bytes, then `n` as 8 big-endian bytes. */
function nonceOf(n) {
  const iv = new Uint8Array(12);
  new DataView(iv.buffer).setBigUint64(4, BigInt(n));
  return iv;
}

/**
 * The extension's side of an authenticated connection. Seal and open one
 * message at a time, in order: the numbering stops replays.
 *
 * @returns {Promise<{seal: (text: string) => Promise<string>, open: (envelope: object) => Promise<string>}>}
 */
export async function createSession(token, server, client) {
  const outgoing = await directionKey(token, "rpp-extension-to-app", server, client);
  const incoming = await directionKey(token, "rpp-app-to-extension", server, client);
  let sent = 0;
  let expected = 0;
  return {
    async seal(text) {
      const n = sent++;
      const data = await crypto.subtle.encrypt({ name: "AES-GCM", iv: nonceOf(n) }, outgoing, encoder.encode(text));
      return JSON.stringify({ type: "sealed", n, data: toBase64(new Uint8Array(data)) });
    },
    async open(envelope) {
      if (envelope?.type !== "sealed" || envelope.n !== expected) {
        throw new Error("a sealed message is out of order");
      }
      const plain = await crypto.subtle.decrypt(
        { name: "AES-GCM", iv: nonceOf(envelope.n) },
        incoming,
        fromBase64(envelope.data),
      );
      expected += 1;
      return decoder.decode(plain);
    },
  };
}
