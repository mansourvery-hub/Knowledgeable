import 'dart:developer' as dev;
import 'package:flutter/material.dart';
import 'package:flutter_chat_types/flutter_chat_types.dart' as types;
import 'package:flutter_chat_ui/flutter_chat_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'chat_message_adapter.dart';
import '../../data/models/message.dart' as model;
import '../../state/chat_controller.dart';
import '../../state/messages_provider.dart';

class ChatScreen extends ConsumerStatefulWidget {
  const ChatScreen({super.key, required this.conversationId});

  final String conversationId;

  @override
  ConsumerState<ChatScreen> createState() => _ChatScreenState();
}

class _ChatScreenState extends ConsumerState<ChatScreen> {
  late final types.User _user;
  late final types.User _tutor;

  @override
  void initState() {
    super.initState();
    _user = const types.User(id: 'user');
    _tutor = const types.User(
      id: 'tutor',
      firstName: 'AI Learning',
      lastName: 'Tutor',
    );
  }

  types.Message _mapToFlyerMessage(model.Message msg) {
    return ChatMessageAdapter.toFlyerMessage(msg, _user, _tutor);
  }

  void _handleSendPressed(types.PartialText message) {
    dev.log('_handleSendPressed called with: ${message.text}');
    final notifier = ref.read(chatControllerProvider(widget.conversationId).notifier);
    
    notifier.send(message.text);
  }

  @override
  Widget build(BuildContext context) {
    final messagesAsync = ref.watch(messagesProvider(widget.conversationId));
    final chatState = ref.watch(chatControllerProvider(widget.conversationId));

    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Scaffold(
      backgroundColor: colorScheme.surface,
      appBar: AppBar(
        title: const Text('Knowledgeable Chat'),
        leading: IconButton(
          icon: Icon(Icons.arrow_back_ios_new, size: 20, color: colorScheme.onSurface),
          onPressed: () => context.go('/'),
        ),
      ),
      body: messagesAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, st) => Center(child: SelectableText('Error: $e')),
        data: (messages) {
    // Map local domain messages to Flyer Chat message types
    final flyerMessages = messages.map(_mapToFlyerMessage).toList().reversed.toList();

    return Column(
      children: [
        Expanded(
          child: Chat(
            messages: flyerMessages,
            onSendPressed: _handleSendPressed,
            user: _user,
            theme: DefaultChatTheme(
              backgroundColor: colorScheme.surface,
              primaryColor: colorScheme.primary,
              secondaryColor: colorScheme.surfaceContainerHighest,
              inputBackgroundColor: colorScheme.surfaceContainerHighest,
              inputTextColor: colorScheme.onSurface,
            ),
            bubbleBuilder: (child, {required message, required nextMessageInGroup}) {
              return Container(
                padding: const EdgeInsets.all(12),
                margin: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  color: message.author.id == _user.id 
                      ? colorScheme.primary 
                      : colorScheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(16),
                ),
                child: SelectableText(
                  message is types.TextMessage ? message.text : '',
                  style: TextStyle(
                    color: message.author.id == _user.id 
                        ? colorScheme.onPrimary 
                        : colorScheme.onSurfaceVariant,
                  ),
                ),
              );
            },
          ),
        ),
        if (chatState.toolName != null)
          Container(
            color: colorScheme.surface,
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: Row(
              children: [
                SizedBox(
                  width: 14,
                  height: 14,
                  child: CircularProgressIndicator(
                    strokeWidth: 2,
                    color: colorScheme.primary,
                  ),
                ),
                const SizedBox(width: 12),
                Text(
                  'Tutor is using ${chatState.toolName}...',
                  style: theme.textTheme.bodyMedium?.copyWith(
                    fontStyle: FontStyle.italic,
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
        if (chatState.isStreaming && chatState.toolName == null)
          Container(
            color: colorScheme.surface,
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: Row(
              children: [
                SizedBox(
                  width: 14,
                  height: 14,
                  child: CircularProgressIndicator(
                    strokeWidth: 2,
                    color: colorScheme.primary,
                  ),
                ),
                const SizedBox(width: 12),
                Text(
                  'Tutor is formulating response...',
                  style: theme.textTheme.bodyMedium?.copyWith(
                    fontStyle: FontStyle.italic,
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
      ],
    );
        },
      ),
    );
  }
}
