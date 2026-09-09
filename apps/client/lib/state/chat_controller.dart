import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/api/sse_parser.dart';
import '../data/models/message.dart';
import '../data/repositories/conversation_repository.dart';
import 'messages_provider.dart';

class ChatState {
  const ChatState({this.isStreaming = false, this.error, this.streamingText = ''});
  final bool isStreaming;
  final String? error;
  final String streamingText;

  ChatState copyWith({bool? isStreaming, String? error, String? streamingText}) => ChatState(
        isStreaming: isStreaming ?? this.isStreaming,
        error: error,
        streamingText: streamingText ?? this.streamingText,
      );
}

class ChatController extends FamilyNotifier<ChatState, String> {
  StreamSubscription<SseEnvelope>? _sub;

  @override
  ChatState build(String conversationId) {
    ref.onDispose(() {
      _sub?.cancel();
    });
    return const ChatState();
  }

  Future<void> send(String content) async {
    if (content.trim().isEmpty || state.isStreaming) return;

    final conversationId = arg;
    state = state.copyWith(isStreaming: true, error: null, streamingText: '');

    final repo = ref.read(conversationRepositoryProvider);
    final messagesNotifier = ref.read(messagesProvider(conversationId).notifier);

    // Optimistic user message
    final userMsg = Message(
      id: 'local-${DateTime.now().millisecondsSinceEpoch}',
      conversationId: conversationId,
      role: 'user',
      content: content,
      createdAt: DateTime.now(),
    );
    messagesNotifier.addLocal(userMsg);

    // Placeholder assistant message for streaming
    final assistantPlaceholder = Message(
      id: 'assistant-${DateTime.now().millisecondsSinceEpoch}',
      conversationId: conversationId,
      role: 'assistant',
      content: '',
      createdAt: DateTime.now(),
    );
    messagesNotifier.addLocal(assistantPlaceholder);

    try {
      final stream = repo.streamMessage(conversationId: conversationId, content: content);
      _sub = stream.listen(
        (envelope) {
          if (envelope.event == 'text_delta') {
            final text = envelope.data['text'] as String? ?? '';
            // update placeholder
            messagesNotifier.updateLastAssistant(text, append: true);
            state = state.copyWith(streamingText: state.streamingText + text);
          } else if (envelope.event == 'turn_completed') {
            // done — refresh from server to get authoritative messages (replaces local ids)
            ref.invalidate(messagesProvider(conversationId));
          } else if (envelope.event == 'error') {
            final msg = envelope.data['message'] as String? ?? 'unknown error';
            state = state.copyWith(error: msg);
          }
        },
        onError: (Object e, StackTrace st) {
          state = state.copyWith(error: e.toString(), isStreaming: false);
        },
        onDone: () {
          state = state.copyWith(isStreaming: false);
          // Ensure final state is persisted; refresh once more after stream ends
          Future.delayed(const Duration(milliseconds: 100), () {
            ref.invalidate(messagesProvider(conversationId));
          });
        },
        cancelOnError: false,
      );

      await _sub?.asFuture();
    } catch (e) {
      state = state.copyWith(error: e.toString(), isStreaming: false);
    } finally {
      state = state.copyWith(isStreaming: false);
    }
  }

  void retryLast(String content) {
    send(content);
  }
}

final chatControllerProvider =
    NotifierProviderFamily<ChatController, ChatState, String>(ChatController.new);
