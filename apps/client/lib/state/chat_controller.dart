import 'dart:async';
import 'dart:developer' as dev;
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/api/sse_parser.dart';
import '../data/models/message.dart';
import '../data/repositories/conversation_repository.dart';
import 'messages_provider.dart';

class ChatState {
  const ChatState({
    this.isStreaming = false,
    this.error,
    this.toolName,
  });
  final bool isStreaming;
  final String? error;
  final String? toolName;

  ChatState copyWith({bool? isStreaming, String? error, String? toolName}) => ChatState(
        isStreaming: isStreaming ?? this.isStreaming,
        error: error,
        toolName: toolName,
      );
}

class ChatController extends FamilyNotifier<ChatState, String> {
  StreamSubscription<SseEnvelope>? _sub;
  String _pendingAssistantText = '';
  Timer? _flushTimer;
  Timer? _watchdogTimer; // New: Watchdog to prevent stream stalling

  @override
  ChatState build(String conversationId) {
    ref.onDispose(() {
      _sub?.cancel();
      _flushTimer?.cancel();
      _watchdogTimer?.cancel();
    });
    return const ChatState();
  }

  void _resetWatchdog(MessagesNotifier notifier) {
    _watchdogTimer?.cancel();
    _watchdogTimer = Timer(const Duration(seconds: 30), () {
      if (state.isStreaming) {
        dev.log('Watchdog: Stream stalled! Resetting state.');
        _flushDelta(notifier);
        state = state.copyWith(isStreaming: false, error: 'Stream stalled. Check connection.');
        _sub?.cancel();
      }
    });
  }

  void _flushDelta(MessagesNotifier notifier) {
    if (_pendingAssistantText.isNotEmpty) {
      notifier.updateLastAssistant(_pendingAssistantText, append: true);
      _pendingAssistantText = '';
    }
    _flushTimer?.cancel();
    _flushTimer = null;
  }

  void handleSseEvent(SseEnvelope envelope, MessagesNotifier messagesNotifier) {
    dev.log('SSE event received: ${envelope.event}, data: ${envelope.data}');
    
    if (envelope.event == 'text_delta') {
      final text = envelope.data['text'] as String? ?? '';
      _pendingAssistantText += text;
      _flushTimer ??= Timer(const Duration(milliseconds: 33), () => _flushDelta(messagesNotifier));
    } else if (envelope.event == 'tool_call_started') {
      state = state.copyWith(toolName: envelope.data['tool_name'] as String?);
    } else if (envelope.event == 'tool_call_finished') {
      state = state.copyWith(toolName: null);
    } else if (envelope.event == 'turn_completed') {
      _flushDelta(messagesNotifier);
      ref.invalidate(messagesProvider(arg));
      state = state.copyWith(toolName: null);
    } else if (envelope.event == 'error') {
      _flushDelta(messagesNotifier);
      final msg = envelope.data['message'] as String? ?? 'unknown error';
      state = state.copyWith(error: msg, toolName: null);
    }
  }

  Future<void> send(String content) async {
    dev.log('ChatController.send: Attempting stream, current state.isStreaming: ${state.isStreaming}');
    if (content.trim().isEmpty || state.isStreaming) {
      dev.log('ChatController.send: Blocked! content empty? ${content.trim().isEmpty}, isStreaming? ${state.isStreaming}');
      return;
    }
    
    state = state.copyWith(isStreaming: true, error: null);
    
    final conversationId = arg;
    final repo = ref.read(conversationRepositoryProvider);
    final messagesNotifier = ref.read(messagesProvider(conversationId).notifier);
    
    _resetWatchdog(messagesNotifier); // Start watchdog on send
    dev.log('ChatController.send: Set isStreaming to true');

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
      
      // Use a completer to wait for the stream to finish properly
      final completer = Completer<void>();
      
      _sub = stream.listen(
        (envelope) {
          _resetWatchdog(messagesNotifier);
          handleSseEvent(envelope, messagesNotifier);
        },
        onError: (Object e, StackTrace st) {
          dev.log('SSE CRITICAL ERROR: $e\nStackTrace: $st');
          _flushDelta(messagesNotifier);
          state = state.copyWith(error: e.toString(), isStreaming: false);
          if (!completer.isCompleted) completer.complete();
        },
        onDone: () {
          dev.log('SSE stream done');
          _flushDelta(messagesNotifier);
          state = state.copyWith(isStreaming: false);
          if (!completer.isCompleted) completer.complete();
        },
        cancelOnError: false,
      );

      await completer.future;
    } catch (e) {
      _flushDelta(messagesNotifier);
      state = state.copyWith(error: e.toString(), isStreaming: false);
    } finally {
      // Ensure we clean up the subscription
      await _sub?.cancel();
      _sub = null;
      state = state.copyWith(isStreaming: false);
      dev.log('ChatController.send: finally block reached, isStreaming=false');
    }
  }


  void retryLast(String content) {
    send(content);
  }
}

final chatControllerProvider =
    NotifierProviderFamily<ChatController, ChatState, String>(ChatController.new);
