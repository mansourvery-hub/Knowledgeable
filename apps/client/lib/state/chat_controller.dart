import 'dart:async';
import 'dart:developer' as dev;
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/api/sse_parser.dart';
import '../data/models/message.dart';
import '../data/repositories/conversation_repository.dart';
import 'messages_provider.dart';

class ChatState {
  const ChatState({this.isStreaming = false, this.error});
  final bool isStreaming;
  final String? error;

  ChatState copyWith({bool? isStreaming, String? error}) => ChatState(
        isStreaming: isStreaming ?? this.isStreaming,
        error: error,
      );
}

class ChatController extends FamilyNotifier<ChatState, String> {
  StreamSubscription<SseEnvelope>? _sub;
  String _pendingAssistantText = '';
  Timer? _flushTimer;

  @override
  ChatState build(String conversationId) {
    ref.onDispose(() {
      _sub?.cancel();
      _flushTimer?.cancel();
    });
    return const ChatState();
  }

  void _flushDelta(MessagesNotifier notifier) {
    if (_pendingAssistantText.isNotEmpty) {
      notifier.updateLastAssistant(_pendingAssistantText, append: true);
      _pendingAssistantText = '';
    }
    _flushTimer?.cancel();
    _flushTimer = null;
  }

  Future<void> send(String content) async {
    dev.log('ChatController.send called with: $content');
    if (content.trim().isEmpty || state.isStreaming) {
      dev.log('ChatController.send: early return. isEmpty: ${content.trim().isEmpty}, isStreaming: ${state.isStreaming}');
      return;
    }

    final conversationId = arg;
    state = state.copyWith(isStreaming: true, error: null);

    final repo = ref.read(conversationRepositoryProvider);
    final messagesNotifier = ref.read(messagesProvider(conversationId).notifier);

    final userMsg = Message(
      id: 'local-${DateTime.now().millisecondsSinceEpoch}',
      conversationId: conversationId,
      role: 'user',
      content: content,
      createdAt: DateTime.now(),
    );
    messagesNotifier.addLocal(userMsg);

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
            _pendingAssistantText += text;
            _flushTimer ??= Timer(const Duration(milliseconds: 33), () => _flushDelta(messagesNotifier));
          } else if (envelope.event == 'turn_completed') {
            _flushDelta(messagesNotifier);
            ref.invalidate(messagesProvider(conversationId));
          } else if (envelope.event == 'error') {
            _flushDelta(messagesNotifier);
            final msg = envelope.data['message'] as String? ?? 'unknown error';
            state = state.copyWith(error: msg);
          }
        },
        onError: (Object e, StackTrace st) {
          _flushDelta(messagesNotifier);
          state = state.copyWith(error: e.toString(), isStreaming: false);
        },
        onDone: () {
          _flushDelta(messagesNotifier);
          state = state.copyWith(isStreaming: false);
        },
        cancelOnError: false,
      );

      await _sub?.asFuture();
    } catch (e) {
      _flushDelta(messagesNotifier);
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
