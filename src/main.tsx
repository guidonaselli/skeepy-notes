import { render } from "solid-js/web";
import App from "./App";
import NoteWindow from "./NoteWindow";

const root = document.getElementById("root");
if (!root) throw new Error("No #root element found");

const params = new URLSearchParams(window.location.search);
const noteId = params.get("note");

if (noteId) {
  render(() => <NoteWindow noteId={noteId} />, root);
} else {
  render(() => <App />, root);
}
