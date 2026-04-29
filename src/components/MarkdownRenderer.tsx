import { type Component, createMemo } from "solid-js";
import { marked } from "marked";
import DOMPurify from "dompurify";

interface Props {
  content: string;
  class?: string;
}

marked.setOptions({ breaks: true, gfm: true });

export const MarkdownRenderer: Component<Props> = (props) => {
  const html = createMemo(() =>
    DOMPurify.sanitize(marked.parse(props.content) as string)
  );

  return (
    <div
      class={`markdown-body${props.class ? ` ${props.class}` : ""}`}
      innerHTML={html()}
    />
  );
};
