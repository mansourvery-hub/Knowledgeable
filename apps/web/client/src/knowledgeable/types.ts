/**
 * Shared client-side types for Knowledgeable chat extensions.
 *
 * These mirror the adapter payloads emitted by `crates/api/src/routes/librechat`.
 * Concept highlighting (M7), the Personal Wiki (M8), and the Graph Explorer (M9)
 * build on these definitions.
 */

/** Whether the learner is strong, weak, or new to a highlighted concept. */
export type ConceptAnnotationStatus = 'known' | 'weak' | 'new';

export interface ConceptAnnotation {
  concept_id: string;
  name: string;
  /** Null when the learner has no state for the concept (renders as New). */
  learner_confidence: number | null;
  status: ConceptAnnotationStatus;
}
