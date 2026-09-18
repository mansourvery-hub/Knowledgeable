import { unified } from 'unified';
import remarkParse from 'remark-parse';
import remarkMath from 'remark-math';
import type { Node } from 'unist';
import {
  CONCEPT_HIGHLIGHT_NODE,
  remarkConceptHighlight,
} from '../plugins/remarkConceptHighlight';
import type { ConceptAnnotation } from '../types';

const PRIME: ConceptAnnotation = {
  concept_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
  name: 'Prime Number',
  learner_confidence: 0.98,
  status: 'known',
};
const FACTOR: ConceptAnnotation = {
  concept_id: 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
  name: 'Factor',
  learner_confidence: 0.3,
  status: 'weak',
};

function transform(markdown: string, annotations: ConceptAnnotation[]): Node {
  const processor = unified()
    .use(remarkParse)
    .use(remarkMath)
    .use(remarkConceptHighlight, { annotations });
  const tree = processor.parse(markdown);
  processor.runSync(tree);
  return tree;
}

interface FlatNode {
  type: string;
  value?: string;
  data?: { hName?: string; hProperties?: Record<string, unknown> };
  children?: FlatNode[];
}

function collect(node: FlatNode, out: FlatNode[] = []): FlatNode[] {
  out.push(node);
  for (const child of node.children ?? []) {
    collect(child, out);
  }
  return out;
}

function highlights(tree: Node): FlatNode[] {
  return collect(tree as FlatNode).filter((n) => n.type === CONCEPT_HIGHLIGHT_NODE);
}

describe('remarkConceptHighlight', () => {
  it('wraps known mentions with highlight data', () => {
    const nodes = highlights(transform('A Prime Number and a factor.', [PRIME, FACTOR]));
    expect(nodes).toHaveLength(1);
    expect(nodes[0].data?.hName).toBe(CONCEPT_HIGHLIGHT_NODE);
    expect(nodes[0].data?.hProperties).toMatchObject({
      conceptId: PRIME.concept_id,
      name: 'Prime Number',
      status: 'known',
      confidence: 0.98,
    });
    // Original casing preserved in the rendered text.
    expect(nodes[0].children?.[0].value).toBe('Prime Number');
  });

  it('leaves weak and new mentions as plain text (T25: chat is known-only)', () => {
    const tree = transform('A Prime Number and a factor.', [PRIME, FACTOR]);
    const flat = collect(tree as FlatNode);
    // The weak mention survives verbatim, with no highlight wrapper.
    expect(flat.some((n) => n.type === 'text' && (n.value ?? '').includes('factor'))).toBe(
      true,
    );
    expect(
      highlights(tree).every((n) => n.data?.hProperties?.status === 'known'),
    ).toBe(true);
  });

  it('prefers the longest name on overlap and skips subwords', () => {
    const number: ConceptAnnotation = {
      concept_id: 'c',
      name: 'Number',
      learner_confidence: null,
      status: 'new',
    };
    const nodes = highlights(
      transform('a prime number in the factory', [number, PRIME]),
    );
    expect(nodes).toHaveLength(1);
    expect(nodes[0].children?.[0].value).toBe('prime number');
  });

  it('leaves fenced code, inline code, and math byte-identical', () => {
    const md = '```\nPrime Number\n```\n\n`Factor` and $Prime Number$ and $$Factor$$';
    const tree = transform(md, [PRIME, FACTOR]);
    expect(highlights(tree)).toHaveLength(0);
    const flat = collect(tree as FlatNode);
    expect(flat.some((n) => n.type === 'code' && n.value === 'Prime Number')).toBe(true);
    expect(flat.some((n) => n.type === 'inlineCode' && n.value === 'Factor')).toBe(true);
    expect(
      flat.some((n) => (n.type === 'inlineMath' || n.type === 'math') && (n.value ?? '').includes('Prime Number') || (n.value ?? '').includes('Factor')),
    ).toBe(true);
  });

  it('never touches HTML tags or attributes', () => {
    // Annotated term only inside an attribute: no highlight may be created.
    const tree = transform('<span title="Prime Number">hi</span>', [PRIME]);
    expect(highlights(tree)).toHaveLength(0);
    const flat = collect(tree as FlatNode);
    expect(
      flat.some((n) => n.type === 'html' && (n.value ?? '').includes('title=')),
    ).toBe(true);
  });

  it('passes through without annotations', () => {
    expect(highlights(transform('Prime Number', []))).toHaveLength(0);
    expect(highlights(transform('Prime Number', undefined as never))).toHaveLength(0);
  });

  it('highlights inside emphasis and link labels', () => {
    const nodes = highlights(transform('*Prime Number* and [a Factor](https://x.test)', [PRIME, FACTOR]));
    expect(nodes).toHaveLength(1);
    expect(nodes[0].children?.[0].value).toBe('Prime Number');
  });
});
