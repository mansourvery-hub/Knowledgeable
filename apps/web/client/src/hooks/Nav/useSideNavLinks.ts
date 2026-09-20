import { useMemo } from 'react';
import { MCPIcon, AttachmentIcon, OpenAIMinimalIcon } from '@librechat/client';
import {
  Bot,
  Bookmark,
  Network,
  NotebookPen,
  ScrollText,
  ArrowRightToLine,
  SlidersHorizontal,
} from 'lucide-react';
import {
  Permissions,
  EModelEndpoint,
  PermissionTypes,
  isParamEndpoint,
  isAgentsEndpoint,
  isAssistantsEndpoint,
} from 'librechat-data-provider';
import type { TInterfaceConfig, TEndpointsConfig } from 'librechat-data-provider';
import type { NavLink } from '~/common';
import {
  useAgentCapabilities,
  useMCPServerManager,
  useGetAgentsConfig,
  useHasAccess,
} from '~/hooks';
import MCPBuilderPanel from '~/components/SidePanel/MCPBuilder/MCPBuilderPanel';
import AgentPanelSwitch from '~/components/SidePanel/Agents/AgentPanelSwitch';
import BookmarkPanel from '~/components/SidePanel/Bookmarks/BookmarkPanel';
import GraphPanel from '~/knowledgeable/components/GraphPanel';
import PanelSwitch from '~/components/SidePanel/Builder/PanelSwitch';
import Parameters from '~/components/SidePanel/Parameters/Panel';
import FilesPanel from '~/components/SidePanel/Files/Panel';
import { attachmentsDisabled } from '~/knowledgeable/attachments';
import { PromptsAccordion } from '~/components/Prompts';
import { SkillsAccordion } from '~/components/Skills';

export default function useSideNavLinks({
  hidePanel,
  keyProvided,
  endpoint,
  endpointType,
  interfaceConfig,
  endpointsConfig,
  includeHidePanel = true,
}: {
  hidePanel?: () => void;
  keyProvided: boolean;
  endpoint?: EModelEndpoint | null;
  endpointType?: EModelEndpoint | null;
  interfaceConfig: Partial<TInterfaceConfig>;
  endpointsConfig: TEndpointsConfig;
  includeHidePanel?: boolean;
}) {
  const hasAccessToPrompts = useHasAccess({
    permissionType: PermissionTypes.PROMPTS,
    permission: Permissions.USE,
  });
  const hasAccessToSkills = useHasAccess({
    permissionType: PermissionTypes.SKILLS,
    permission: Permissions.USE,
  });
  const hasAccessToBookmarks = useHasAccess({
    permissionType: PermissionTypes.BOOKMARKS,
    permission: Permissions.USE,
  });
  // No Memories: generic LibreChat memory was deleted with this change
  // (policy §9.6) — the graph/learner state stays authoritative.
  const hasAccessToAgents = useHasAccess({
    permissionType: PermissionTypes.AGENTS,
    permission: Permissions.USE,
  });
  const hasAccessToCreateAgents = useHasAccess({
    permissionType: PermissionTypes.AGENTS,
    permission: Permissions.CREATE,
  });
  const hasAccessToUseMCPSettings = useHasAccess({
    permissionType: PermissionTypes.MCP_SERVERS,
    permission: Permissions.USE,
  });
  const hasAccessToCreateMCP = useHasAccess({
    permissionType: PermissionTypes.MCP_SERVERS,
    permission: Permissions.CREATE,
  });
  // No Schedules: the panel directory was deleted with this change.
  const { availableMCPServers } = useMCPServerManager();

  const { agentsConfig } = useGetAgentsConfig({ endpointsConfig });
  const { skillsEnabled } = useAgentCapabilities(agentsConfig?.capabilities);

  const Links = useMemo(() => {
    const links: NavLink[] = [];

    if (
      endpointsConfig?.[EModelEndpoint.agents] &&
      hasAccessToAgents &&
      hasAccessToCreateAgents &&
      endpointsConfig[EModelEndpoint.agents].disableBuilder !== true
    ) {
      links.push({
        title: 'com_sidepanel_agent_builder',
        label: '',
        icon: Bot,
        id: EModelEndpoint.agents,
        Component: AgentPanelSwitch,
      });
    }

    if (
      isAssistantsEndpoint(endpoint) &&
      ((endpoint === EModelEndpoint.assistants &&
        endpointsConfig?.[EModelEndpoint.assistants] &&
        endpointsConfig[EModelEndpoint.assistants].disableBuilder !== true) ||
        (endpoint === EModelEndpoint.azureAssistants &&
          endpointsConfig?.[EModelEndpoint.azureAssistants] &&
          endpointsConfig[EModelEndpoint.azureAssistants].disableBuilder !== true)) &&
      keyProvided
    ) {
      links.push({
        title: 'com_sidepanel_assistant_builder',
        label: '',
        icon: OpenAIMinimalIcon,
        id: EModelEndpoint.assistants,
        Component: PanelSwitch,
      });
    }

    if (hasAccessToSkills && skillsEnabled) {
      links.push({
        title: 'com_ui_skills',
        label: '',
        icon: ScrollText,
        id: 'skills',
        Component: SkillsAccordion,
      });
    }

    // Knowledgeable: scheduled chats were deleted with this change (policy
    // §9.3) — no `/api/schedules*` backend exists.

    if (hasAccessToPrompts) {
      links.push({
        title: 'com_ui_prompts',
        label: '',
        icon: NotebookPen,
        id: 'prompts',
        Component: PromptsAccordion,
      });
    }

    if (hasAccessToBookmarks) {
      links.push({
        title: 'com_sidepanel_conversation_tags',
        label: '',
        icon: Bookmark,
        id: 'bookmarks',
        Component: BookmarkPanel,
      });
    }

    // Knowledgeable graph explorer (T9/M9): read-only neighborhood inspection
    // beside chat, which remains the primary surface. Always available in the
    // single-user local mode; no permission gate fits this local graph view.
    links.push({
      title: 'com_ui_knowledge_graph',
      label: '',
      icon: Network,
      id: 'knowledge-graph',
      Component: GraphPanel,
    });

    // Knowledgeable: file attachments are FUTURE (no `/api/files/*` backend),
    // so the Files panel would list nothing and offer dead uploads.
    if (!attachmentsDisabled) {
      links.push({
        title: 'com_sidepanel_attach_files',
        label: '',
        icon: AttachmentIcon,
        id: 'files',
        Component: FilesPanel,
      });
    }

    if (
      interfaceConfig.parameters === true &&
      isParamEndpoint(endpoint ?? '', endpointType ?? '') === true &&
      !isAgentsEndpoint(endpoint) &&
      keyProvided
    ) {
      links.push({
        title: 'com_sidepanel_parameters',
        label: '',
        icon: SlidersHorizontal,
        id: 'parameters',
        Component: Parameters,
      });
    }

    if (
      (hasAccessToUseMCPSettings && availableMCPServers && availableMCPServers.length > 0) ||
      hasAccessToCreateMCP
    ) {
      links.push({
        title: 'com_nav_setting_mcp',
        label: '',
        icon: MCPIcon,
        id: 'mcp-builder',
        Component: MCPBuilderPanel,
      });
    }

    if (includeHidePanel && hidePanel) {
      links.push({
        title: 'com_sidepanel_hide_panel',
        label: '',
        icon: ArrowRightToLine,
        onClick: hidePanel,
        id: 'hide-panel',
      });
    }

    return links;
  }, [
    endpoint,
    endpointsConfig,
    keyProvided,
    hasAccessToAgents,
    hasAccessToCreateAgents,
    hasAccessToPrompts,
    hasAccessToSkills,
    skillsEnabled,
    interfaceConfig.parameters,
    endpointType,
    hasAccessToBookmarks,
    availableMCPServers,
    hasAccessToUseMCPSettings,
    hasAccessToCreateMCP,
    includeHidePanel,
    hidePanel,
  ]);

  return Links;
}
