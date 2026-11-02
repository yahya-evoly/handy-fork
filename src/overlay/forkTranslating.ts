// FORK: whether the current dictation is being translated (see
// src-tauri/src/fork_language.rs `finish`). The working pill then reads
// "Translating..." instead of "Transcribing...", in both the Minimal and the
// Live overlay. Module-level so no event is missed while the pill remounts.
import { listen } from "@tauri-apps/api/event";
import { useSyncExternalStore } from "react";

let translating = false;
const subscribers = new Set<() => void>();

listen<boolean>("fork-translating", (event) => {
  translating = event.payload;
  subscribers.forEach((notify) => notify());
});

const subscribe = (notify: () => void) => {
  subscribers.add(notify);
  return () => {
    subscribers.delete(notify);
  };
};

export const useForkTranslating = () =>
  useSyncExternalStore(subscribe, () => translating);
