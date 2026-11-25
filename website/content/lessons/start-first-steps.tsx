import Link from "next/link";
import { Terminal } from "@/components/terminal";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import versions from "@/data/versions.json";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P } from "./parts";

// ---------- What a Program Is ----------

export const programDoc: LessonDoc = {
  lead: "A program is a list of steps for the computer, written in a language both you and the computer can read.",
  simple: <>A program is like a recipe: steps written in order, which the computer follows exactly, very fast, and nothing more. Before anything runs, Kavach reads the whole recipe and tells you about any mistake it finds.</>,
  minutes: 4,
  body: (
    <>
      <P>
        A computer does exactly what it is told, very fast, and nothing else. A <strong className="text-ink">program</strong>{" "}
        is the telling: steps written down in order. This is a whole Kavach program:
      </P>
      <Ex name="start/script" />
      <P>
        Each line is one step. The first prints a greeting. The second gives the number 2026 a name,{" "}
        <C>year</C>. The third prints a sentence with that number inside it. The box under the code shows what the
        program printed when it ran.
      </P>
      <H2>From text to running program</H2>
      <ol className="m-0 flex list-decimal flex-col gap-1.5 pl-5">
        <li>You write the steps in a text file, the <strong className="text-ink">source code</strong>. Kavach files end in <C>.kav</C>, and are saved as UTF-8 text, as most editors save by default. The invisible mark some Windows editors put at the start of a file (a byte order mark) is fine: Kavach reads it as a space.</li>
        <li>The <strong className="text-ink">compiler</strong> reads the file and checks it. If something is wrong, it tells you what, and where.</li>
        <li>If all is well, it turns the file into instructions the processor runs directly.</li>
      </ol>
      <Note>
        You don&apos;t need to know how a processor works. You write the steps; Kavach checks them and makes them run.
      </Note>
    </>
  ),
  quiz: [
    {
      type: "bool",
      prompt: <>The compiler runs your program even when it finds a mistake, and the mistake shows up later.</>,
      answer: false,
      explain: "Kavach checks the whole file first. A mistake it can find is reported before anything runs.",
    },
  ],
};

// ---------- The Terminal ----------

export const terminalDoc: LessonDoc = {
  lead: "The terminal is a window where you type commands. You need four of them to run Kavach programs.",
  simple: <>The terminal is a way to talk to your computer by typing instead of clicking. You type a short command, press Enter, and the computer does it, like giving a helper instructions one sentence at a time.</>,
  minutes: 5,
  body: (
    <>
      <P>
        On Windows, open <strong className="text-ink">Terminal</strong> or <strong className="text-ink">PowerShell</strong>{" "}
        from the Start menu. On macOS, open <strong className="text-ink">Terminal</strong>; on Linux, your terminal app.
        A line with a blinking cursor appears: type a command there and press Enter.
      </P>
      <H2>The commands you need</H2>
      <Terminal
        commands
        session={"cd projects # go into the folder named projects\ncd .. # go back up one folder\nls # list the files here (dir works too on Windows)\nkavach run hello.kav # check hello.kav, then run it"}
      />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>cd</C> moves you into a folder: run your program from the folder its file is in.</li>
        <li>Press the Up arrow to bring back the last command, instead of typing it again.</li>
        <li>Press Ctrl+C to stop a program that keeps running.</li>
      </ul>
      <Note>
        An editor such as VS Code has a terminal built in (View › Terminal), already in your project&apos;s folder.
      </Note>
    </>
  ),
};

// ---------- Install ----------
//
// The requirements: platforms from the roadmap; build tools from
// rust-toolchain.toml and decisions 0016, 0114 and 0124. A built hello
// program on Windows was checked to load only Windows' own DLLs.

const requirements: [string, React.ReactNode][] = [
  [
    "Use Kavach",
    <>Windows 10 or 11, or Linux, on a 64-bit Intel or AMD processor (x86-64). WSL works. On macOS you can build it yourself, untested so far (<a href="#on-macos">below</a>); full support for macOS and ARM processors (Apple silicon, Raspberry Pi) comes in Phase 3 of the <a href={`${versions.repo}/blob/main/docs/vision/roadmap.md`}>roadmap</a>.</>,
  ],
  [
    "Build Kavach",
    <><a href="https://rustup.rs">Rust</a>, through rustup: the repository&apos;s <C>rust-toolchain.toml</C> picks the exact version and its <C>llvm-tools</C>, and rustup fetches both on the first build. Also git, and <a href="https://cmake.org/download/">CMake</a>, which builds the gamepad library.</>,
  ],
  [
    "…and a C compiler",
    <>Rust needs one too. On Windows, the Visual Studio Build Tools with <strong className="text-ink">Desktop development with C++</strong>. On Linux, <C>build-essential</C> (or <C>gcc</C>), <C>pkg-config</C>, and the sound library&apos;s headers: <C>libasound2-dev</C> on Debian and Ubuntu.</>,
  ],
  ["kavach run, test, check", <>Nothing more.</>],
  ["kavach build", <>A linker: the same C compiler tools. Kavach says so if they are missing.</>],
  [
    "Run a built program",
    <>No Kavach: its runtime is inside the executable. On Windows it loads only Windows&apos; own libraries. On Linux, a program that plays sound loads <C>libasound.so.2</C>, which every desktop Linux has.</>,
  ],
  [
    "Use a GPU, windows, sound",
    <>A GPU with Vulkan or DirectX 12 drivers (NVIDIA, AMD or Intel), a screen (X11 or Wayland on Linux), a sound device. Gamepads need nothing installed.</>,
  ],
];

export const installDoc: LessonDoc = {
  lead: "One clone, one build. A single-command installer comes with the first public release.",
  simple: <>Installing puts Kavach on your computer so you can use it. For now you download its source code and build it yourself, a bit like putting together a flat-pack table. It takes a few minutes, and you do it once.</>,
  minutes: 5,
  before: ["start/the-terminal"],
  body: (
    <>
      <H2>System requirements</H2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">To…</th>
              <th className="py-2 font-bold">You need</th>
            </tr>
          </thead>
          <tbody>
            {requirements.map(([what, need]) => (
              <tr key={what} className="border-b border-line align-top">
                <td className="py-2 pr-4 whitespace-nowrap">{what}</td>
                <td className="py-2 text-muted">{need}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>
        Without a GPU, a screen or a sound device, the functions that need one fail with an error your program can
        handle, such as <C>Unavailable</C>, and a machine with no gamepad has no pads. Nothing crashes.
      </P>
      <Note>
        Only want to try it? The <Link href="/playground/">playground</Link> runs the real compiler in your browser,
        with nothing to install.
      </Note>
      <H2>Build it</H2>
      <P>
        Kavach is young and moves fast, so today you build it from source, with the tools above. Without CMake, the
        build stops at once and says how to install it on your system.
      </P>
      <Terminal commands session={`git clone ${versions.repo}\ncd kavach\ncargo build --release -p kavach-cli`} />
      <P>
        The build takes a few minutes, once. The program lands at <C>target/release/kavach</C>: add that folder to
        your PATH, and <C>kavach</C> works in every terminal.
      </P>
      <H2>On macOS</H2>
      <P>
        Releases are built for Windows and Linux only. On a Mac you can build Kavach yourself, on Apple silicon or Intel:
      </P>
      <Terminal
        commands
        session={`xcode-select --install\nbrew install cmake\ncurl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh\nrustup component add llvm-tools\ngit clone ${versions.repo}\ncd kavach\ncargo build --release -p kavach-cli -p kavach-runtime-native`}
      />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>The first line installs Apple&apos;s command-line tools: the C compiler and linker <C>kavach build</C> needs. The second installs CMake through <a href="https://brew.sh">Homebrew</a>, which builds the gamepad library, as on every system. The third installs Rust.</li>
        <li>The program lands at <C>target/release/kavach</C>, as on the other systems.</li>
        <li>Without <C>llvm-tools</C>, Kavach still builds programs, through Cranelift, but they run slower.</li>
        <li>Windows (<C>std.window</C>) and drawing on the GPU aren&apos;t available on macOS yet.</li>
      </ul>
      <Note>
        Nothing on macOS is tested yet. If something fails, or works, say so in the project&apos;s{" "}
        <a href={`${versions.repo}/discussions`}>Discussions</a>: it helps decide what comes first.
      </Note>
      <H2>Your editor</H2>
      <P>
        The VS Code extension in <C>editors/vscode</C> adds colors, errors as you type, and go-to-definition.
      </P>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="rust-toolchain.toml" label="the Rust version" />
        <SrcRef file="docs/architecture/windows-setup.md" label="setting up Windows" />
        <SrcRef file="docs/decisions/0016-native-runtime-and-executables.md" label="decision 0016 · executables and linking" />
        <SrcRef file="docs/decisions/0114-sound-through-cpal.md" label="decision 0114 · sound on Linux" />
        <SrcRef file="docs/decisions/0124-gamepads-through-sdl3.md" label="decision 0124 · CMake" />
      </div>
    </>
  ),
  quiz: [
    {
      type: "choice",
      prompt: <>You wrote hello.kav. Which command runs it?</>,
      options: ["kavach run hello.kav", "kavach hello.kav --exec", "kavach compile hello.kav"],
      answer: 0,
      explain: "run checks the file and runs it at once; build makes an executable instead.",
    },
  ],
};

// ---------- Reading Errors ----------

export const errorsIntroDoc: LessonDoc = {
  lead: "Error messages are help: each one says what is wrong, where, and usually what to write instead.",
  simple: <>An error message is like a note from a careful proofreader: it points to the exact spot, says what is wrong, and often suggests the fix. Read it as help, not as a complaint.</>,
  minutes: 5,
  before: ["start/your-first-program"],
  body: (
    <>
      <P>Everyone writes mistakes, every day. Kavach finds most of them before the program runs:</P>
      <Ex name="errors-intro/typo.fails" />
      <P>
        In the terminal, the message goes on: a line <C>--&gt; typo.kav:3:11</C> names the file, the line and the
        column, and the line itself is shown with <C>^^^^</C> under the mistake. Read the first line first: it is
        usually the fix.
      </P>
      <H2>Types that don&apos;t fit</H2>
      <Ex name="errors-intro/types.fails" />
      <P>
        <C>age + 1</C> is a number, but <C>next</C> was declared as text. Decide which you meant: a number, or{" "}
        <C>String(age + 1)</C>.
      </P>
      <H2>Panics: stops while running</H2>
      <P>
        A few mistakes can only show while the program runs, such as a loop that counts one past the end of a list
        when the count comes from somewhere else. The program stops at once, with the reason, instead of carrying on
        with a wrong value:
      </P>
      <Ex name="errors-intro/panic.panics" />
      <P>
        When the compiler knows both the list and the range, the same loop is a compile error instead: see{" "}
        <Link href="/docs/errors/kinds-of-errors/">Kinds of Errors</Link>.
      </P>
      <Note>
        When a message isn&apos;t clear, that is a bug in Kavach: <Link href={`${versions.repo}/issues`}>tell us</Link>.
      </Note>
    </>
  ),
  tryIt: {
    task: <>Fix the first program above so it prints 42.</>,
    answer: <P>Spell the name as it was declared: <C>print(total + 2)</C>.</P>,
  },
};
