import 'dart:developer' as dev;
import 'package:flutter/material.dart';
import 'package:flutter_chat_types/flutter_chat_types.dart' as types;
import 'package:flutter_chat_ui/flutter_chat_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

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
    return types.TextMessage(
      author: msg.isUser ? _user : _tutor,
      createdAt: msg.createdAt.millisecondsSinceEpoch,
      id: msg.id,
      text: msg.content,
    );
  }

  void _handleSendPressed(types.PartialText message) {
    dev.log('_handleSendPressed called with: ${message.text}');
    ref.read(chatControllerProvider(widget.conversationId).notifier).send(message.text);
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
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'AI Learning Tutor',
              style: theme.textTheme.titleMedium?.copyWith(
                fontWeight: FontWeight.bold,
                color: colorScheme.onSurface,
              ),
            ),
            Text(
              'Teaching from your frontier',
              style: theme.textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
              ),
            ),
          ],
        ),
        leading: IconButton(
          icon: Icon(Icons.arrow_back_ios_new, size: 20, color: colorScheme.onSurface),
          onPressed: () => context.go('/'),
        ),
        actions: [
          IconButton(
            icon: Icon(Icons.refresh, color: colorScheme.onSurface),
            onPressed: () => ref.invalidate(messagesProvider(widget.conversationId)),
          ),
        ],
        elevation: 0,
        backgroundColor: colorScheme.surface,
        bottom: PreferredSize(
          preferredSize: const Size.fromHeight(1),
          child: Container(
            color: colorScheme.outlineVariant.withValues(alpha: 0.5),
            height: 1,
          ),
        ),
      ),
      body: Column(
        children: [
          Expanded(
            child: messagesAsync.when(
              loading: () => const Center(child: CircularProgressIndicator()),
              error: (e, st) => Center(
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Text('Failed to load messages: $e', style: theme.textTheme.bodyMedium),
                    const SizedBox(height: 12),
                    ElevatedButton(
                      onPressed: () => ref.invalidate(messagesProvider(widget.conversationId)),
                      style: ElevatedButton.styleFrom(
                        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
                      ),
                      child: const Text('Retry'),
                    ),
                  ],
                ),
              ),
              data: (messages) {
                // Map local domain messages to Flyer Chat message types
                final flyerMessages = messages.map(_mapToFlyerMessage).toList().reversed.toList();

                return Chat(
                  messages: flyerMessages,
                  onSendPressed: _handleSendPressed,
                  user: _user,
                  showUserAvatars: false,
                  showUserNames: false,
                  theme: DefaultChatTheme(
                    backgroundColor: colorScheme.surface,
                    primaryColor: colorScheme.primary,
                    secondaryColor: colorScheme.surfaceContainerHighest.withValues(alpha: 0.7),
                    inputBackgroundColor: colorScheme.surfaceContainerHighest.withValues(alpha: 0.4),
                    inputTextColor: colorScheme.onSurface,
                    inputBorderRadius: const BorderRadius.all(Radius.circular(28)),
                    inputTextStyle: theme.textTheme.bodyMedium ?? const TextStyle(),
                    sendButtonIcon: Icon(Icons.send_rounded, color: colorScheme.primary),
                    sentMessageBodyTextStyle: theme.textTheme.bodyMedium?.copyWith(
                          color: colorScheme.onPrimary,
                        ) ??
                        const TextStyle(),
                    receivedMessageBodyTextStyle: theme.textTheme.bodyMedium?.copyWith(
                          color: colorScheme.onSurface,
                        ) ??
                        const TextStyle(),
                  ),
                );
              },
            ),
          ),
          if (chatState.isStreaming)
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
          if (chatState.error != null)
            Container(
              width: double.infinity,
              color: colorScheme.errorContainer,
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
              child: Row(
                children: [
                  Icon(Icons.error_outline, size: 20, color: colorScheme.onErrorContainer),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Text(
                      chatState.error!,
                      style: theme.textTheme.bodyMedium?.copyWith(
                        color: colorScheme.onErrorContainer,
                      ),
                    ),
                  ),
                  TextButton(
                    onPressed: chatState.isStreaming
                        ? null
                        : () => ref
                            .read(chatControllerProvider(widget.conversationId).notifier)
                            .retryLast(''),
                    child: Text(
                      'RETRY',
                      style: theme.textTheme.labelLarge?.copyWith(
                        color: colorScheme.onErrorContainer,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                  ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}
