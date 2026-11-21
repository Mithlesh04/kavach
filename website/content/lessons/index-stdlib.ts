// The Standard Library's pages, by course key: spread into the lessons of
// index.ts, which would pass the 500-line limit with them. lib/sample-pages.ts
// reads this file too.

import type { LessonDoc } from "@/lib/chapter";
import { base64Doc, cryptoDoc } from "./stdlib-crypto";
import { compressDoc } from "./stdlib-compress";
import { windowDoc } from "./stdlib-window";
import { processDoc } from "./stdlib-process";
import { systemDoc } from "./stdlib-system";
import { gamepadDoc } from "./stdlib-input";
import { soundDoc } from "./stdlib-audio";
import { vecDoc } from "./stdlib-vec";
import { imageDoc } from "./stdlib-image";
import { appDoc } from "./stdlib-app";
import { fontDoc } from "./stdlib-font";
import { reflectDoc } from "./stdlib-reflect";
import { tensorDoc } from "./stdlib-tensor";
import { tensorIndexDoc, tensorMakeDoc } from "./stdlib-tensor-make";
import { tensorMasksDoc, tensorMathDoc } from "./stdlib-tensor-math";
import { tensorReduceDoc, tensorSortDoc } from "./stdlib-tensor-reduce";
import { tensorFftDoc, tensorLinalgDoc } from "./stdlib-tensor-linalg";
import { tableCsvDoc, tableDoc } from "./stdlib-table";
import { loggingDoc } from "./stdlib-log";
import { gpuScreenDoc } from "./stdlib-gpu";
import { textCollectionsDoc } from "./stdlib-text";
import { jsonBuildingDoc, jsonDecodingDoc, jsonDoc, jsonNestedDoc, jsonReadingDoc, jsonValuesDoc } from "./stdlib-json";
import { bytesDoc } from "./stdlib-bytes";
import {
  argumentsDoc, cliDoc, commandsDoc, fileInfoDoc, filesDoc, foldersDoc, readingDoc, watchDoc, writingDoc,
} from "./stdlib-files";
import { clipboardDoc, colorsDoc, devicesDoc, promptsDoc, randomDoc, regexDoc, termDoc, timeDoc } from "./stdlib-tools";
import { agentsStdDoc } from "./stdlib-agents";

export const stdlibLessons: Record<string, LessonDoc> = {
  "standard-library/files": filesDoc,
  "standard-library/files/reading": readingDoc,
  "standard-library/files/writing": writingDoc,
  "standard-library/files/folders": foldersDoc,
  "standard-library/files/details-and-permissions": fileInfoDoc,
  "standard-library/files/watching-for-changes": watchDoc,
  "standard-library/command-line-programs": cliDoc,
  "standard-library/command-line-programs/arguments": argumentsDoc,
  "standard-library/command-line-programs/commands": commandsDoc,
  "standard-library/the-terminal": termDoc,
  "standard-library/the-terminal/colors-and-styles": colorsDoc,
  "standard-library/the-terminal/prompts": promptsDoc,
  "standard-library/time": timeDoc,
  "standard-library/random-numbers": randomDoc,
  "standard-library/regex": regexDoc,
  "standard-library/text-and-collections": textCollectionsDoc,
  "standard-library/logging": loggingDoc,
  "standard-library/agents": agentsStdDoc,
  "standard-library/clipboard": clipboardDoc,
  "standard-library/devices": devicesDoc,
  "standard-library/cryptography": cryptoDoc,
  "standard-library/base64": base64Doc,
  "standard-library/compression": compressDoc,
  "standard-library/windows": windowDoc,
  "standard-library/running-programs": processDoc,
  "standard-library/gpu-screens": gpuScreenDoc,
  "standard-library/the-machine": systemDoc,
  "standard-library/gamepads": gamepadDoc,
  "standard-library/sound": soundDoc,
  "standard-library/vectors": vecDoc,
  "standard-library/images": imageDoc,
  "standard-library/desktop-apps": appDoc,
  "standard-library/fonts": fontDoc,
  "standard-library/reflection": reflectDoc,
  "standard-library/tensors": tensorDoc,
  "standard-library/tensors/making-tensors": tensorMakeDoc,
  "standard-library/tensors/indexing": tensorIndexDoc,
  "standard-library/tensors/arithmetic-and-broadcasting": tensorMathDoc,
  "standard-library/tensors/masks": tensorMasksDoc,
  "standard-library/tensors/reductions": tensorReduceDoc,
  "standard-library/tensors/sorting-and-joining": tensorSortDoc,
  "standard-library/tensors/linear-algebra": tensorLinalgDoc,
  "standard-library/tensors/the-fft": tensorFftDoc,
  "standard-library/tables": tableDoc,
  "standard-library/tables/csv": tableCsvDoc,
  "standard-library/json": jsonDoc,
  "standard-library/json/json-values": jsonValuesDoc,
  "standard-library/json/nested-data": jsonNestedDoc,
  "standard-library/json/reading": jsonReadingDoc,
  "standard-library/json/building": jsonBuildingDoc,
  "standard-library/json/decoding-into-a-type": jsonDecodingDoc,
  "standard-library/bytes": bytesDoc,
};
