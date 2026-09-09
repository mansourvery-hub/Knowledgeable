import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../data/models/conversation.dart';
import '../data/repositories/conversation_repository.dart';

class ConversationsNotifier extends AsyncNotifier<List<Conversation>> {
  @override
  Future<List<Conversation>> build() async {
    final repo = ref.read(conversationRepositoryProvider);
    return repo.list();
  }

  Future<Conversation> create({String? title}) async {
    final repo = ref.read(conversationRepositoryProvider);
    final conv = await repo.create(title: title);
    // optimistic update
    final current = state.valueOrNull ?? [];
    state = AsyncData([conv, ...current]);
    return conv;
  }

  Future<void> refresh() async {
    state = const AsyncLoading();
    state = await AsyncValue.guard(() async {
      final repo = ref.read(conversationRepositoryProvider);
      return repo.list();
    });
  }
}

final conversationsProvider =
    AsyncNotifierProvider<ConversationsNotifier, List<Conversation>>(
  ConversationsNotifier.new,
);
