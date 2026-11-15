import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { specBlockWith } from "@/lib/spec-tables";
import { Ex, H2, Note, P } from "./parts";
import { ModuleTable } from "./spec-table";

// ---------- Sound ----------
//
// Spec 14.27. The site's checks must never make a sound, so playing is
// shown from the spec; making and reading sounds needs nothing, so those
// samples run, and the compile errors are checked.

const FILE = "14-standard-library/27-audio.md";
const TESTS = "tests/conformance/14-standard-library";

export const soundDoc: LessonDoc = {
  lead: "std.audio plays sound: effects that start the moment something happens, and music. A sound is data until it plays, so loading or making one needs nothing; playing needs Audio.",
  simple: <>A sound is a long list of numbers: how far a speaker moves, many thousand times a second. Kavach lets a program read those numbers from a file, or make its own, and play them. The program goes on while the sound plays.</>,
  minutes: 5,
  before: ["agents/capabilities", "standard-library/files"],
  body: (
    <>
      <P>
        Load two sounds, play music on a loop, and an effect on the left. This is the spec&apos;s example. It makes a
        sound, so it is shown here, not run:
      </P>
      <CodeBlock code={specBlockWith(FILE, "14.27", "audio.play(music")} file="main.kav, from spec 14.27" />
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
        <li>Generated tests skip functions that use <C>Audio</C>, so a test run plays nothing.</li>
      </ul>
      <H2>Every function and method</H2>
      <ModuleTable file="27-audio.md" number="14.27" first="Function or method" />
      <H2>Not yet</H2>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>Recording, behind the <C>Microphone</C> capability: what is said near the machine is private.</li>
        <li>Sounds a Kavach function makes as they play, for synthesizers.</li>
        <li>Streaming long music from a file, choosing the output device, and Opus files.</li>
      </ul>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file={`docs/spec/${FILE}`} label="spec 14.27 · std.audio" />
        <SrcRef file={`${TESTS}/sounds-are-data-until-played.kav`} label="sounds as data, tested" />
        <SrcRef file={`${TESTS}/playing-sound-needs-audio.kav`} label="Audio, tested" />
        <SrcRef file="stdlib/audio/audio.kav" label="audio.kav" />
        <SrcRef file="runtime/audio/src/mixer.rs" label="the mixer" />
        <SrcRef file="docs/decisions/0114-sound-through-cpal.md" label="decision 0114" />
      </div>
    </>
  ),
};
