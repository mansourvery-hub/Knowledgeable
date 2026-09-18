import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:knowledgeable_client/state/chat_controller.dart';
import 'package:knowledgeable_client/state/messages_provider.dart';
import 'package:knowledgeable_client/core/api/sse_parser.dart';

void main() {
  group('ChatController', () {
    test('Tool state updates based on SSE events', () {
      final container = ProviderContainer();
      final messagesNotifier = container.read(messagesProvider('test-conv').notifier);
      final controller = container.read(chatControllerProvider('test-conv').notifier);
      
      // Simulate tool_call_started
      final toolEvent = const SseEnvelope(version: 1, event: 'tool_call_started', data: {'tool_name': 'find_concept'});
      controller.handleSseEvent(toolEvent, messagesNotifier);
      
      // Verify state update
      expect(container.read(chatControllerProvider('test-conv')).toolName, 'find_concept');

      // Simulate tool_call_finished
      final toolFinishedEvent = const SseEnvelope(version: 1, event: 'tool_call_finished', data: {});
      controller.handleSseEvent(toolFinishedEvent, messagesNotifier);
      
      expect(container.read(chatControllerProvider('test-conv')).toolName, isNull);
    });
  });
}
