/**
 * Overlay host for Knowledgeable reading surfaces (Phase 3, SPEC 7.3).
 *
 * Mounted once in `ChatRoute`; renders `WikiDrawer` today and any future
 * overlays. Keeps the upstream seam to a single component swap.
 */
import WikiDrawer from './WikiDrawer';

export default function KnowledgeableHost() {
  return <WikiDrawer />;
}
