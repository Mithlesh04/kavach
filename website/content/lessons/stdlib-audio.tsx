import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specTables } from "@/lib/spec-tables";
import { Ex, H2, Note, P, SpecCode } from "./parts";
import { Table } from "./spec-table";

// ---------- Sound ----------
//
// Spec 14.27. The site's checks must never make a sound or listen to the
// room, so playing and recording are shown from the spec; making and
// reading sounds needs nothing, so those samples run, and the compile
// errors are checked. Both tables start "Function or method", so each is
// picked by its place and checked by its first row.

const FILE = "14-standard-library/27-audio.md";
const TESTS = "tests/conformance/14-standard-library";

function AudioTable({ n, first }: { n: number; first: string }) {
  const t = specTables(FILE, "14.27")[n];
  if (!t || !t.rows[0][0].includes(first)) throw new Error(`spec 14.27: table ${n} no longer starts with ${first}`);
  return <Table header={t.header} rows={t.rows} wrapFirst />;
}

export const soundDoc: LessonDoc = {
  lead: "std.audio plays sound, effects and music, and records what the microphone hears. A sound is data until it plays, so loading or making one needs nothing; playing needs Audio, and recording needs Microphone.",
  simple: <>A sound is a long list of numbers: how far a speaker moves, many thousand times a second. Kavach lets a program read those numbers from a file, or make its own, and play them. The program goes on while the sound plays.</>,
  minutes: 5,
  before: ["agents/capabilities", "standard-library/files"],
  body: (
    <>
      <P>
        Load two sounds, play music on a loop, and an effect on the left. This is the spec&apos;s example. It makes a
        sound, so it is shown here, not run:
      </P>
      <SpecCode file={FILE} number="14.27" text={"audio.play(music"} name="main.kav" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>audio.decode(bytes)</C> reads a WAV or an Ogg Vorbis file, told apart by their first bytes.</li>
        <li><C>audio.play</C> starts the sound and gives a <C>Voice</C> at once: the program doesn&apos;t wait.</li>
        <li><C>volume</C> 0 is silent and 1 the sound as it is; <C>pan</C> -1 is left and 1 right; <C>looped</C> plays it over and over.</li>
        <li>The <C>Voice</C> changes the sound while it plays: <C>set_volume</C>, <C>set_pan</C>, <C>stop</C>.</li>
      </ul>
      <Note>
        The runtime mixes every playing sound on the sound device&apos;s own thread. So a slow frame, or a pause in the
        program, never makes the sound click.
      </Note>
      <H2>Long music: streaming</H2>
      <P>
        <C>audio.decode</C> holds the whole sound in memory, which is right for short effects. For long music,{" "}
        <C>audio.stream(path)</C> plays a WAV or Ogg Vorbis file while reading it a piece at a time. It takes{" "}
        <C>volume</C>, <C>pan</C> and <C>looped</C> as <C>play</C> does, and gives a <C>Voice</C> the same way. It reads
        a file, so it needs <C>Fs.read</C> as well as <C>Audio</C>.
      </P>
      <P>
        It opens the file before anything plays, so a file that is missing, or isn&apos;t a sound, is{" "}
        <C>BadSound</C>, even on a machine with no sound device:
      </P>
      <Ex name="audio/stream-missing" />
      <H2>Voices you fill</H2>
      <P>
        <C>audio.voice()</C> gives a voice whose sound the program makes itself, a frame at a time: a synthesizer, or an
        engine&apos;s hum that follows its speed. Each frame of the program&apos;s loop, <C>voice.fill(state, next)</C>{" "}
        calls <C>next</C> for every sound frame the voice has room for, about 50 ms ahead. This plays a tone, so it is
        shown here, not run:
      </P>
      <SpecCode file={FILE} number="14.27" text={"audio.voice()"} name="tone.kav" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>next</C> takes the state and gives the state after the frame, then its left and right, as <C>Float32</C>s.</li>
        <li>The state goes from one call to the next, so a function can keep a phase, a filter or an envelope.</li>
        <li>The voice plays 48,000 frames a second. If the program stops calling <C>fill</C>, it goes quiet when its queue runs out, and starts again when called.</li>
      </ul>
      <H2>A sound made from numbers</H2>
      <P>
        <C>audio.sound(samples, rate, channels)</C> makes a sound from a list of <C>Float32</C>s, each from -1 to 1.
        This one is a beep of 440 Hz, the A that orchestras tune to, as a square wave: up for half of each wave, down
        for the other half. Making it plays nothing, so it runs:
      </P>
      <Ex name="audio/tone" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>rate</C> is frames a second, 1,000 to 384,000. A frame is one sample of each channel.</li>
        <li>With 2 channels, left and right take turns: left, right, left, right.</li>
        <li><C>frames</C>, <C>rate</C> and <C>channels</C> are fields; <C>seconds()</C> is how long it lasts.</li>
      </ul>
      <P>So a stereo sound needs an even number of samples. Three can&apos;t be stereo:</P>
      <Ex name="audio/odd-stereo.panics" />
      <H2>When it fails</H2>
      <P>
        <C>AudioError</C> has two cases. <C>BadSound(reason)</C>: the bytes aren&apos;t a sound file Kavach reads.{" "}
        <C>Unavailable(reason)</C>: nothing can play, as on a server with no sound device, or with 256 voices playing
        already.
      </P>
      <Ex name="audio/not-a-sound" />
      <H2>Playing needs Audio</H2>
      <P>
        Sound reaches the people near the machine, so playing it is a capability. A <C>pub</C> function that plays
        must say <C>uses Audio</C>:
      </P>
      <Ex name="audio/needs-audio.fails" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Letting a <C>Voice</C> go doesn&apos;t stop it, so <C>audio.play(boom)</C> alone plays the whole sound.</li>
        <li>Letting a <C>Sound</C> go while it plays plays it to its end.</li>
        <li>Voices add up. Where they add past 1, the sound is held at 1.</li>
        <li>Generated tests skip functions that use <C>Audio</C> or <C>Microphone</C>, so a test run plays and records nothing.</li>
      </ul>
      <H2>Recording</H2>
      <P>
        <C>audio.record(seconds)</C> records a clip and waits for it. <C>audio.listen()</C> starts recording and gives
        a <C>Recording</C> at once; each <C>take()</C> gives what was heard since the one before, so a loop can show a
        level meter or send the voice on. It records the room, so it is shown here, not run:
      </P>
      <SpecCode file={FILE} number="14.27" text={"audio.listen("} name="main.kav" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>It records the system&apos;s default microphone, at its own rate and with its first two channels.</li>
        <li>What it hears is a <C>Sound</C>, like any other: play it, or read its samples.</li>
        <li>Between takes it keeps up to 60 seconds; past that, what it hears is dropped until the next take.</li>
        <li><C>rec.stop()</C> stops it, and letting a <C>Recording</C> go stops it too.</li>
      </ul>
      <AudioTable n={1} first="audio.listen()" />
      <P>
        What is said near the machine is private, so recording is a capability of its own. A <C>pub</C> function that
        records must say <C>uses Microphone</C>:
      </P>
      <Ex name="audio/needs-microphone.fails" />
      <H2>Every function and method</H2>
      <AudioTable n={0} first="audio.decode" />
      <H2>Not yet</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Choosing the microphone, and its rate, instead of the system&apos;s default.</li>
        <li>A voice whose sound a Kavach function makes on the sound device&apos;s own thread, with no time ahead.</li>
        <li>Choosing the output device, and Opus files.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.27 · std.audio" />
        <SrcRef file={`${TESTS}/sounds-are-data-until-played.kav`} label="sounds as data, tested" />
        <SrcRef file={`${TESTS}/playing-sound-needs-audio.kav`} label="Audio, tested" />
        <SrcRef file={`${TESTS}/recording-needs-microphone.kav`} label="Microphone, tested" />
        <SrcRef file={`${TESTS}/streaming-sound-reads-its-file.kav`} label="streaming, tested" />
        <SrcRef file={`${TESTS}/a-missing-stream-is-bad-sound.kav`} label="a missing stream, tested" />
        <SrcRef file="stdlib/audio/audio.kav" label="audio.kav" />
        <SrcRef file="runtime/audio/src/mixer.rs" label="the mixer" />
        <SrcRef file="docs/decisions/0114-sound-through-cpal.md" label="decision 0114" />
      </div>
    </>
  ),
};
