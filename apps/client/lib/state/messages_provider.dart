import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../data/models/message.dart';
import '../data/repositories/conversation_repository.dart';

class MessagesNotifier extends FamilyAsyncNotifier<List<Message>, String> {
  @override
  Future<List<Message>> build(String conversationId) async {
    final repo = ref.read(conversationRepositoryProvider);
    return repo.listMessages(conversationId);
  }

  Future<void> refresh() async {
    final conversationId = arg;
    state = const AsyncLoading();
    state = await AsyncValue.guard(() async {
      final repo = ref.read(conversationRepositoryProvider);
      return repo.listMessages(conversationId);
    });
  }

  void addLocal(Message msg) {
    final current = state.valueOrNull ?? [];
    state = AsyncData([...current, msg]);
  }

  void updateLastAssistant(String textDelta, {bool append = true}) {
    final current = state.valueOrNull ?? [];
    if (current.isEmpty) return;
    final last = current.last;
    if (!last.isAssistant) return;
    final updated = Message(
      id: last.id,
      conversationId: last.conversationId,
      role: last.role,
      content: append ? last.content + textDelta : textDelta,
      createdAt: last.createdAt,
    );
    state = AsyncData([...current.sublist(0, current.length - 1), updated]);
  }
}

final messagesProvider =
    AsyncNotifierProviderFamily<MessagesNotifier, List<Message>, String>(
  MessagesNotifier.new,
);
