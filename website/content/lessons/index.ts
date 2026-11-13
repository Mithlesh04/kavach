// The one-idea pages (decision 0047), by course key. A key in
// lib/course.ts without an entry here or in content/chapters stays off
// the site until it is written.

import type { LessonDoc } from "@/lib/chapter";
import { breakDoc, whileDoc, whileEndlessDoc, whilePlainDoc } from "./basics-while";
import { forDoc, forIndexDoc, forListDoc, forMapDoc, forRangeDoc, forTimesDoc } from "./basics-for";
import { commentsDoc, debugDoc, outputDoc, printDoc, valuesPrintDoc } from "./basics-output";
import { blocksDoc, keywordsDoc, mainFunctionDoc, namesDoc, sourceFilesDoc, syntaxDoc } from "./basics-syntax";
import { changeDoc, constDoc, destructuringDoc, letDoc, variablesDoc } from "./basics-variables";
import { firstProgramDoc, homeDoc, smartDoc } from "./start";
import { editionsDoc, envDoc, layoutDoc, settingsDoc } from "./tools-settings";
import { taggedLiteralsDoc } from "./tools-literals";
import { limitsDoc } from "./limits";
import { chainsDoc, classesDoc, extendDoc } from "./types-oop";
import { agentModelDoc } from "./agents-model";
import { agentSettingsDoc } from "./agents-settings";
import { deprecateDoc } from "./functions-deprecate";
import { mistakesDoc } from "./start-mistakes";
import { classicDoc } from "./start-classic";
import { messagesRefDoc } from "./reference-messages";
import { commandsRefDoc, keywordsRefDoc, literalsRefDoc, methodsRefDoc, operatorsRefDoc, stdlibRefDoc } from "./reference";
import { bridgeDoc, cppDoc, goDoc, javascriptDoc, pythonDoc, rustDoc } from "./start-bridges";
import { buildDoc, whoDoc } from "./start-who";
import { whyDoc } from "./start-why";
import { arenasDoc, automaticDoc, fastDoc, sharedDoc, unsafeDoc, valuesDoc } from "./memory";
import { freedDoc } from "./memory-freed";
import { errorKindsDoc } from "./errors-kinds";
import { monitorDoc } from "./agents-monitor";
import { practiceDoc } from "./agents-practice";
import {
  auditDoc as kavachAuditDoc, callingCDoc, cCallbacksDoc, cNumbersDoc, cRecordsDoc, cTextDoc, libraryDoc,
} from "./other-languages";
import {
  coresDoc, howFastDoc, nativeDoc, packedDoc, radixDoc, sizesDoc,
} from "./speed";
import {
  aiToolsDoc, contextDoc as aiContextDoc, mcpDoc, modulesDoc, outlineDoc, packagesDoc, pubDoc, queryDoc, smallFilesDoc, useDoc, workspacesDoc,
} from "./tools-project";
import {
  buildDoc as kavachBuildDoc, checkDoc, commandDoc, editorDoc, fixDoc, formatterDoc, newDoc, runDoc, testDoc,
} from "./tools-cli";
import {
  channelsDoc, deadlockDoc, leakDoc, manyDoc, putTakeDoc, racesDoc, sendersDoc, taskFailsDoc, tasksDoc,
  timeoutsDoc as taskTimeoutsDoc,
} from "./concurrency";
import { howTasksRunDoc, turnsDoc } from "./concurrency-running";
import { httpCallDoc, httpDoc, httpJsonDoc, httpServeDoc, httpSessionDoc, httpSocketsDoc } from "./networks-http";
import { base64Doc, cryptoDoc } from "./stdlib-crypto";
import { compressDoc } from "./stdlib-compress";
import { textCollectionsDoc } from "./stdlib-text";
import {
  grpcClientStreamDoc, grpcDoc, grpcProtoDoc, grpcServerStreamDoc, grpcTwoWayDoc, socketsDoc, tcpDoc, timeoutsDoc,
  tlsDoc, udpDoc,
} from "./networks-net";
import { sortingDoc, sortKeyDoc, sortNumbersDoc, sortTextDoc, sortTwoKeysDoc } from "./collections-sort";
import { gpuCanDoc, gpuDoc, gpuFnDoc, gpuMapDoc, gpuRecordsDoc, gpuSameDoc, gpuUsesDoc, gpuWhereDoc } from "./speed-gpu";
import { gpuSumsDoc } from "./gpu-streams";
import {
  bytesDoc, jsonBuildingDoc, jsonDecodingDoc, jsonDoc, jsonNestedDoc, jsonReadingDoc, jsonValuesDoc,
} from "./stdlib-json";
import {
  argumentsDoc, cliDoc, commandsDoc, fileInfoDoc, filesDoc, foldersDoc, readingDoc, writingDoc,
} from "./stdlib-files";
import {
  clipboardDoc, colorsDoc, devicesDoc, loggingDoc, promptsDoc, randomDoc, regexDoc, termDoc, timeDoc,
} from "./stdlib-tools";
import { agentsStdDoc } from "./stdlib-agents";
import {
  capabilitiesDoc, declareCapDoc, fsNarrowDoc, implementCapDoc, ownCapsDoc, pureDoc, refusedCapDoc, usesDoc,
} from "./agents-capabilities";
import {
  agentsDoc, canDoc, checksOrderDoc, declareAgentDoc, limitDoc, onDoc, runningDoc,
} from "./agents-core";
import { approveDoc } from "./agents-approve";
import { durableDoc, stateDoc } from "./agents-durable";
import { aiFailsDoc, askDoc, modelDoc, typedAnswersDoc } from "./agents-ai";
import {
  auditDoc, dryRunDoc, enforceDoc, modesDoc, observeDoc, suggestDoc,
} from "./agents-trust";
import { revealDoc, secretFieldsDoc, secretParamsDoc, secretResultsDoc, secretsDoc } from "./agents-secrets";
import { useCasesDoc } from "./agents-uses";
import {
  catchDoc, contextDoc, errorCatchAllDoc, errorsDoc, matchErrorsDoc, panicsDoc, panicsWhatDoc, stopsDoc, tryDoc,
} from "./errors";
import {
  checkedDoc, contractsDoc, ensureDoc, exampleDoc, fakesDoc, fromTypesDoc, invariantDoc, ownValuesDoc, requireDoc, testBlocksDoc,
  testingDoc,
} from "./contracts";
import {
  anonDoc, declareRecordDoc, declareUnionDoc, defaultsRecordDoc, genericsDoc, interfacesDoc, interfaceValuesDoc,
  matchUnionDoc, methodsDoc, optionalReadDoc, optionalsDoc, optionalWhatDoc, recordsDoc, unionsDoc, withDoc,
  writingTypesDoc,
} from "./types";
import {
  addRemoveDoc, changeListDoc, changeMapDoc, checkSetDoc, listsDoc, makeListDoc, makeMapDoc, makeSetDoc, mapsDoc,
  pipelineDoc, readListDoc, readMapDoc, setsDoc, sortListDoc, tuplesDoc, walkMapDoc,
} from "./collections";
import {
  declaringDoc, defaultsDoc, dotDoc, fnValuesDoc, functionsDoc, lambdasDoc, namedArgsDoc, namedDoc, parametersDoc,
  recursionDoc, resultsDoc, returningDoc, signatureDoc, writingLambdaDoc,
} from "./functions";
import { inoutDoc } from "./functions-inout";
import { errorsIntroDoc, installDoc, programDoc, terminalDoc } from "./start-first-steps";
import { askNumberDoc, askPasswordDoc, askTextDoc, inputDoc } from "./basics-input";
import { bigintDoc, boolDoc, charDoc, convertDoc, dataTypesDoc, decimalDoc, floatDoc, intDoc, sizedDoc } from "./basics-types";
import { interpolationDoc, joinDoc, lengthDoc, makeStringsDoc, searchDoc, splitDoc, stringsDoc, toNumbersDoc } from "./basics-strings";
import {
  arithmeticDoc, assignmentDoc, booleansDoc, comparisonDoc, exactDoc, logicalDoc, mathDoc, mathFunctionsDoc,
  operatorsDoc, precedenceDoc, roundingDoc,
} from "./basics-operators";
import { elseIfDoc, guardsDoc, ifDoc, ifLetDoc, ifOnlyDoc, ifValueDoc, matchDoc, matchValuesDoc, patternsDoc } from "./basics-if";

export const lessons: Record<string, LessonDoc> = {
  "reference/keywords": keywordsRefDoc,
  "reference/operators": operatorsRefDoc,
  "reference/literals": literalsRefDoc,
  "reference/built-in-methods": methodsRefDoc,
  "reference/standard-library": stdlibRefDoc,
  "reference/commands": commandsRefDoc,
  "reference/compiler-messages": messagesRefDoc,
  "start/kavach-home": homeDoc,
  "start/why-kavach": whyDoc,
  "start/what-kavach-does-for-you": smartDoc,
  "start/classic-mistakes-fixed": classicDoc,
  "start/mistakes-caught-before-a-run": mistakesDoc,
  "start/who-kavach-is-for": whoDoc,
  "start/coming-from-another-language": bridgeDoc,
  "start/coming-from-another-language/python": pythonDoc,
  "start/coming-from-another-language/javascript": javascriptDoc,
  "start/coming-from-another-language/cpp": cppDoc,
  "start/coming-from-another-language/rust": rustDoc,
  "start/coming-from-another-language/go": goDoc,
  "start/what-you-can-build": buildDoc,
  "start/what-a-program-is": programDoc,
  "start/the-terminal": terminalDoc,
  "start/install": installDoc,
  "start/your-first-program": firstProgramDoc,
  "start/reading-errors": errorsIntroDoc,

  "basics/syntax": syntaxDoc,
  "basics/syntax/source-files": sourceFilesDoc,
  "basics/syntax/the-main-function": mainFunctionDoc,
  "basics/syntax/blocks-are-values": blocksDoc,
  "basics/syntax/names": namesDoc,
  "basics/syntax/keywords": keywordsDoc,
  "basics/output": outputDoc,
  "basics/output/print": printDoc,
  "basics/output/how-values-print": valuesPrintDoc,
  "basics/output/debug": debugDoc,
  "basics/comments": commentsDoc,
  "basics/variables": variablesDoc,
  "basics/variables/const": constDoc,
  "basics/variables/let": letDoc,
  "basics/variables/changing-a-value": changeDoc,
  "basics/variables/destructuring": destructuringDoc,
  "basics/user-input": inputDoc,
  "basics/user-input/ask-for-text": askTextDoc,
  "basics/user-input/ask-for-a-number": askNumberDoc,
  "basics/user-input/ask-for-a-password": askPasswordDoc,
  "basics/data-types": dataTypesDoc,
  "basics/data-types/int": intDoc,
  "basics/data-types/sized-integers": sizedDoc,
  "basics/data-types/float-and-float32": floatDoc,
  "basics/data-types/decimal": decimalDoc,
  "basics/data-types/bigint": bigintDoc,
  "basics/data-types/bool": boolDoc,
  "basics/data-types/char": charDoc,
  "basics/data-types/converting-types": convertDoc,
  "basics/operators": operatorsDoc,
  "basics/operators/arithmetic": arithmeticDoc,
  "basics/operators/assignment": assignmentDoc,
  "basics/operators/comparison": comparisonDoc,
  "basics/operators/logical": logicalDoc,
  "basics/operators/precedence": precedenceDoc,
  "basics/strings": stringsDoc,
  "basics/strings/making-strings": makeStringsDoc,
  "basics/strings/interpolation": interpolationDoc,
  "basics/strings/joining": joinDoc,
  "basics/strings/length-and-characters": lengthDoc,
  "basics/strings/searching": searchDoc,
  "basics/strings/splitting": splitDoc,
  "basics/strings/strings-to-numbers": toNumbersDoc,
  "basics/math": mathDoc,
  "basics/math/math-functions": mathFunctionsDoc,
  "basics/math/rounding": roundingDoc,
  "basics/math/exact-numbers": exactDoc,
  "basics/booleans": booleansDoc,
  "basics/if-else": ifDoc,
  "basics/if-else/if": ifOnlyDoc,
  "basics/if-else/else-if": elseIfDoc,
  "basics/if-else/if-as-a-value": ifValueDoc,
  "basics/if-let": ifLetDoc,
  "basics/match": matchDoc,
  "basics/match/matching-values": matchValuesDoc,
  "basics/match/patterns": patternsDoc,
  "basics/match/guards": guardsDoc,
  "basics/for-loop": forDoc,
  "basics/for-loop/over-a-list": forListDoc,
  "basics/for-loop/over-a-range": forRangeDoc,
  "basics/for-loop/over-a-map": forMapDoc,
  "basics/for-loop/with-an-index": forIndexDoc,
  "basics/for-loop/repeat-n-times": forTimesDoc,
  "basics/while-loop": whileDoc,
  "basics/while-loop/while": whilePlainDoc,
  "basics/while-loop/endless-loops": whileEndlessDoc,
  "basics/break-continue": breakDoc,

  "functions/functions": functionsDoc,
  "functions/functions/declaring": declaringDoc,
  "functions/functions/parameters": parametersDoc,
  "functions/functions/returning-a-value": returningDoc,
  "functions/named-arguments": namedDoc,
  "functions/named-arguments/named-arguments": namedArgsDoc,
  "functions/named-arguments/default-values": defaultsDoc,
  "functions/using-results": resultsDoc,
  "functions/lambdas": lambdasDoc,
  "functions/lambdas/writing-a-lambda": writingLambdaDoc,
  "functions/lambdas/dot-shorthand": dotDoc,
  "functions/functions-as-values": fnValuesDoc,
  "functions/recursion": recursionDoc,
  "functions/inout-parameters": inoutDoc,
  "functions/signatures": signatureDoc,
  "functions/deprecating-a-function": deprecateDoc,

  "collections/lists": listsDoc,
  "collections/lists/making-a-list": makeListDoc,
  "collections/lists/reading-by-index": readListDoc,
  "collections/lists/changing-items": changeListDoc,
  "collections/lists/adding-and-removing": addRemoveDoc,
  "collections/lists/sorting": sortListDoc,
  "collections/lists/map-filter-sum": pipelineDoc,
  "collections/maps": mapsDoc,
  "collections/maps/making-a-map": makeMapDoc,
  "collections/maps/reading-a-value": readMapDoc,
  "collections/maps/adding-and-changing": changeMapDoc,
  "collections/maps/looping-over-a-map": walkMapDoc,
  "collections/sets": setsDoc,
  "collections/sets/making-a-set": makeSetDoc,
  "collections/sets/adding-and-checking": checkSetDoc,
  "collections/tuples": tuplesDoc,
  "collections/sorting": sortingDoc,
  "collections/sorting/numbers": sortNumbersDoc,
  "collections/sorting/text": sortTextDoc,
  "collections/sorting/by-a-key": sortKeyDoc,
  "collections/sorting/two-keys": sortTwoKeysDoc,

  "types/records": recordsDoc,
  "types/records/declaring": declareRecordDoc,
  "types/records/default-values": defaultsRecordDoc,
  "types/records/updating-with-with": withDoc,
  "types/methods": methodsDoc,
  "types/method-chains": chainsDoc,
  "types/extend": extendDoc,
  "types/unions": unionsDoc,
  "types/unions/declaring": declareUnionDoc,
  "types/unions/matching-a-union": matchUnionDoc,
  "types/optionals": optionalsDoc,
  "types/optionals/what-an-optional-is": optionalWhatDoc,
  "types/optionals/reading-one-safely": optionalReadDoc,
  "types/anonymous-records": anonDoc,
  "types/generics": genericsDoc,
  "types/interfaces": interfacesDoc,
  "types/interface-values": interfaceValuesDoc,
  "types/classes-and-inheritance": classesDoc,
  "types/writing-types": writingTypesDoc,

  "errors/kinds-of-errors": errorKindsDoc,
  "errors/errors": errorsDoc,
  "errors/try": tryDoc,
  "errors/catch": catchDoc,
  "errors/matching-errors": matchErrorsDoc,
  "errors/the-error-catch-all": errorCatchAllDoc,
  "errors/adding-context": contextDoc,
  "errors/panics": panicsDoc,
  "errors/panics/panics": panicsWhatDoc,
  "errors/panics/when-a-program-stops": stopsDoc,
  "errors/contracts": contractsDoc,
  "errors/contracts/require": requireDoc,
  "errors/contracts/ensure": ensureDoc,
  "errors/contracts/invariant": invariantDoc,
  "errors/contracts/example": exampleDoc,
  "errors/contracts/how-they-are-checked": checkedDoc,
  "errors/testing": testingDoc,
  "errors/testing/tests-from-types": fromTypesDoc,
  "errors/testing/your-own-values": ownValuesDoc,
  "errors/testing/test-blocks": testBlocksDoc,
  "errors/testing/fakes": fakesDoc,

  "memory/values-not-references": valuesDoc,
  "memory/automatic-memory": automaticDoc,
  "memory/automatic-memory/how-it-stays-fast": fastDoc,
  "memory/automatic-memory/freed-immediately": freedDoc,
  "memory/shared-state": sharedDoc,
  "memory/arenas": arenasDoc,
  "memory/unsafe": unsafeDoc,

  "standard-library/files": filesDoc,
  "standard-library/files/reading": readingDoc,
  "standard-library/files/writing": writingDoc,
  "standard-library/files/folders": foldersDoc,
  "standard-library/files/details-and-permissions": fileInfoDoc,
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

  "standard-library/json": jsonDoc,
  "standard-library/json/json-values": jsonValuesDoc,
  "standard-library/json/nested-data": jsonNestedDoc,
  "standard-library/json/reading": jsonReadingDoc,
  "standard-library/json/building": jsonBuildingDoc,
  "standard-library/json/decoding-into-a-type": jsonDecodingDoc,
  "standard-library/bytes": bytesDoc,
  "speed/how-kavach-is-fast": howFastDoc,
  "speed/loops-on-all-cores": coresDoc,
  "speed/byte-packed-lists": packedDoc,
  "speed/fast-sorting": radixDoc,
  "speed/as-data-grows": sizesDoc,
  "speed/native-executables": nativeDoc,
  "speed/gpu": gpuDoc,
  "speed/gpu/gpu-fn": gpuFnDoc,
  "speed/gpu/map-on-the-gpu": gpuMapDoc,
  "speed/gpu/records-and-two-lists": gpuRecordsDoc,
  "speed/gpu/same-results-as-the-cpu": gpuSameDoc,
  "speed/gpu/cpu-or-gpu": gpuWhereDoc,
  "speed/gpu/can-i-use-it-on-the-gpu": gpuCanDoc,
  "speed/gpu/sums-in-a-loop": gpuSumsDoc,
  "speed/gpu/use-cases": gpuUsesDoc,
  "networks/http": httpDoc,
  "networks/http/calling-an-api": httpCallDoc,
  "networks/http/serving": httpServeDoc,
  "networks/http/json-over-http": httpJsonDoc,
  "networks/http/sessions": httpSessionDoc,
  "networks/http/websockets-and-events": httpSocketsDoc,
  "networks/sockets": socketsDoc,
  "networks/sockets/tcp": tcpDoc,
  "networks/sockets/udp": udpDoc,
  "networks/sockets/tls": tlsDoc,
  "networks/sockets/timeouts": timeoutsDoc,
  "networks/grpc": grpcDoc,
  "networks/grpc/server-streaming": grpcServerStreamDoc,
  "networks/grpc/client-streaming": grpcClientStreamDoc,
  "networks/grpc/two-way-and-conversations": grpcTwoWayDoc,
  "networks/grpc/kavach-proto": grpcProtoDoc,
  "concurrency/tasks": tasksDoc,
  "concurrency/tasks/when-a-task-fails": taskFailsDoc,
  "concurrency/tasks-never-leak": leakDoc,
  "concurrency/many-things-at-once": manyDoc,
  "concurrency/channels": channelsDoc,
  "concurrency/channels/put-and-take": putTakeDoc,
  "concurrency/channels/many-senders": sendersDoc,
  "concurrency/channels/deadlocks": deadlockDoc,
  "concurrency/no-data-races": racesDoc,
  "concurrency/timeouts-and-cancellation": taskTimeoutsDoc,
  "concurrency/taking-turns": turnsDoc,
  "concurrency/how-tasks-run": howTasksRunDoc,
  "other-languages/calling-c": callingCDoc,
  "other-languages/calling-c/numbers": cNumbersDoc,
  "other-languages/calling-c/text": cTextDoc,
  "other-languages/calling-c/callbacks": cCallbacksDoc,
  "other-languages/calling-c/records": cRecordsDoc,
  "other-languages/a-library-for-any-language": libraryDoc,
  "other-languages/kavach-audit": kavachAuditDoc,
  "agents/capabilities": capabilitiesDoc,
  "agents/capabilities/uses": usesDoc,
  "agents/capabilities/pure-functions": pureDoc,
  "agents/capabilities/narrower-file-access": fsNarrowDoc,
  "agents/your-own-capabilities": ownCapsDoc,
  "agents/your-own-capabilities/declaring-a-capability": declareCapDoc,
  "agents/your-own-capabilities/implementing-one": implementCapDoc,
  "agents/your-own-capabilities/when-a-call-is-refused": refusedCapDoc,
  "agents/agents": agentsDoc,
  "agents/agents/declaring-an-agent": declareAgentDoc,
  "agents/agents/can": canDoc,
  "agents/agents/limit": limitDoc,
  "agents/agents/approve": approveDoc,
  "agents/agents/model": agentModelDoc,
  "agents/agents/settings": agentSettingsDoc,
  "agents/agents/the-order-of-checks": checksOrderDoc,
  "agents/agents/on-events": onDoc,
  "agents/running-an-agent": runningDoc,
  "agents/ask": askDoc,
  "agents/ask/typed-answers": typedAnswersDoc,
  "agents/ask/when-the-ai-fails": aiFailsDoc,
  "agents/ask/choosing-the-model": modelDoc,
  "agents/agent-modes": modesDoc,
  "agents/agent-modes/observe": observeDoc,
  "agents/agent-modes/suggest": suggestDoc,
  "agents/agent-modes/enforce": enforceDoc,
  "agents/agent-modes/dry-runs": dryRunDoc,
  "agents/audit-log": auditDoc,
  "agents/monitoring-agents": monitorDoc,
  "agents/durable-agents": durableDoc,
  "agents/durable-agents/limits-across-restarts": stateDoc,
  "agents/secrets": secretsDoc,
  "agents/secrets/secret-fields": secretFieldsDoc,
  "agents/secrets/secret-parameters": secretParamsDoc,
  "agents/secrets/secret-results": secretResultsDoc,
  "agents/secrets/reveal": revealDoc,
  "agents/agent-use-cases": useCasesDoc,
  "agents/agents-in-practice": practiceDoc,

  "tools/modules": modulesDoc,
  "tools/modules/use": useDoc,
  "tools/modules/pub": pubDoc,
  "tools/packages": packagesDoc,
  "tools/workspaces": workspacesDoc,
  "tools/tagged-literals": taggedLiteralsDoc,
  "tools/the-kavach-command": commandDoc,
  "tools/the-kavach-command/new": newDoc,
  "tools/the-kavach-command/run": runDoc,
  "tools/the-kavach-command/build": kavachBuildDoc,
  "tools/the-kavach-command/test": testDoc,
  "tools/the-kavach-command/check": checkDoc,
  "tools/the-kavach-command/fix": fixDoc,
  "tools/formatter": formatterDoc,
  "tools/editor-setup": editorDoc,
  "tools/small-files": smallFilesDoc,
  "tools/tools-for-ai": aiToolsDoc,
  "tools/tools-for-ai/outline": outlineDoc,
  "tools/tools-for-ai/query": queryDoc,
  "tools/tools-for-ai/context": aiContextDoc,
  "tools/tools-for-ai/mcp": mcpDoc,
  "tools/project-layout": layoutDoc,
  "tools/settings": settingsDoc,
  "tools/environment-variables": envDoc,
  "tools/editions": editionsDoc,
  "tools/limits-and-sizes": limitsDoc,
};
