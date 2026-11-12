import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";
import { ModuleTable } from "./spec-table";

// ---------- Cryptography ----------

const CRYPTO_SPEC = "docs/spec/14-standard-library/19-crypto.md";

export const cryptoDoc: LessonDoc = {
  lead: "std.crypto hashes, signs and seals data, and stores passwords safely. It comes from vetted libraries, and needs no capability.",
  simple: <>Cryptography is like the locks, seals and signatures of the post office: a seal shows a letter was opened, a signature shows who sent it, and a lock keeps it private. <C>std.crypto</C> gives you tested ones, so you never build your own.</>,
  minutes: 5,
  before: ["standard-library/bytes"],
  body: (
    <>
      <H2>Passwords</H2>
      <P>
        Never store a password itself. Store its hash, and check a login against it. <C>hash_password</C> uses
        Argon2id with a new salt each time, so the same password never gives the same text twice:
      </P>
      <Ex name="crypto/passwords" />
      <H2>Sealing data</H2>
      <P>
        <C>encrypt</C> seals data with AES-256-GCM. Only the same key opens it. Another key, or sealed data that was
        changed even by one bit, fails with <C>Tampered</C> instead of giving wrong data. A key of the wrong size
        fails with <C>BadKey</C>:
      </P>
      <Ex name="crypto/seal" />
      <H2>Signatures</H2>
      <P>
        A signature proves who wrote some data. The secret key signs; anyone with the public key can check, but
        can&apos;t sign:
      </P>
      <Ex name="crypto/sign" />
      <H2>Hashes, tokens and comparing</H2>
      <Ex name="crypto/hashes" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>Bytes</C> prints in hexadecimal, so the SHA-256 digest of <C>abc</C> is shown as it is usually written.</li>
        <li><C>hmac_sha256</C> proves a message came from someone who knows the key, as payment webhooks do.</li>
        <li>Compare secrets with <C>crypto.equal</C>, not <C>==</C>: it takes the same time wherever they differ, so a secret can&apos;t be guessed a byte at a time from how long the answer takes.</li>
        <li><C>token()</C> gives 32 random bytes as 43 letters of URL-safe text: a session id or a reset link.</li>
      </ul>
      <H2>Every function</H2>
      <ModuleTable file="19-crypto.md" number="14.19" first="Function" />
      <P><C>CryptoError</C> is <C>BadKey(reason)</C> for a key of the wrong kind or size, or <C>Tampered</C>.</P>
      <Note>
        Secret keys are <C>Bytes</C> to keep out of logs and prompts. Store them as a program stores any{" "}
        <Link href="/docs/agents/secrets/">secret</Link>.
      </Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={CRYPTO_SPEC} label="spec 14.19 · std.crypto" />
        <SrcRef file="tests/conformance/14-standard-library/crypto-hashes-signs-and-seals.kav" label="tested" />
        <SrcRef file="tests/conformance/14-standard-library/crypto-checks-rsa-signatures.kav" label="RSA checks, tested" />
        <SrcRef file="docs/decisions/0093-std-crypto.md" label="decision 0093" />
      </div>
    </>
  ),
};

// ---------- Base64 ----------

export const base64Doc: LessonDoc = {
  lead: "std.base64 writes bytes as text and reads them back, in the standard alphabet or the URL-safe one. Tokens, cookies and email use it.",
  simple: <>Some places only carry plain letters, like a form that takes text only. Base64 turns any bytes into letters, digits and a few signs, so they travel safely, and turns them back into the same bytes at the other end.</>,
  minutes: 3,
  before: ["standard-library/bytes"],
  body: (
    <>
      <Ex name="base64/encode" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>url: true</C> uses <C>-</C> and <C>_</C> instead of <C>+</C> and <C>/</C>, so the text can go in a URL. Tokens such as JWTs use it, with <C>pad: false</C>.</li>
        <li>It needs no capability: it changes nothing outside the program.</li>
      </ul>
      <H2>Strict decoding</H2>
      <P>
        Decoding is strict, so one value has one text, which signed tokens rely on. Padding may be left out, but when
        it is there it must be right:
      </P>
      <Ex name="base64/strict" />
      <ModuleTable file="20-base64.md" number="14.20" first="Function" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/14-standard-library/20-base64.md" label="spec 14.20 · std.base64" />
        <SrcRef file="tests/conformance/14-standard-library/base64-encodes-and-decodes.kav" label="tested" />
      </div>
    </>
  ),
};
