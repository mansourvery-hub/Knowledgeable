import 'package:flutter_test/flutter_test.dart';
import 'package:knowledgeable_client/app/app.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:knowledgeable_client/data/repositories/conversation_repository.dart';
import 'package:knowledgeable_client/data/models/conversation.dart';
import 'package:knowledgeable_client/data/models/message.dart';
import 'package:knowledgeable_client/core/api/sse_parser.dart';

class FakeConversationRepository implements ConversationRepository {
  @override
  Future<Conversation> create({String? title}) async {
    return Conversation(
      id: 'test-id',
      learnerId: 'test-learner',
      title: title,
      createdAt: DateTime.now(),
      updatedAt: DateTime.now(),
    );
  }

  @override
  Future<List<Conversation>> list() async {
    return [];
  }

  @override
  Future<Conversation> get(String id) async {
    return Conversation(
      id: id,
      learnerId: 'test-learner',
      createdAt: DateTime.now(),
      updatedAt: DateTime.now(),
    );
  }

  @override
  Future<List<Message>> listMessages(String conversationId) async {
    return [];
  }

  @override
  Stream<SseEnvelope> streamMessage({
    required String conversationId,
    required String content,
  }) {
    return const Stream.empty();
  }
}

void main() {
  testWidgets('App renders', (WidgetTester tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          conversationRepositoryProvider.overrideWithValue(FakeConversationRepository()),
        ],
        child: const KnowledgeableApp(),
      ),
    );
    expect(find.text('Knowledgeable'), findsOneWidget);
  });
}
