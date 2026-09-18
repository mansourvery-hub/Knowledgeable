import 'dart:async';
import 'dart:convert';
import 'dart:developer' as dev;

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/api/api_client.dart';
import '../../core/api/sse_parser.dart';
import '../../core/api/sse_stream_stub.dart'
    if (dart.library.html) '../../core/api/sse_stream_web.dart'
    if (dart.library.io) '../../core/api/sse_stream_native.dart' as sse_impl;
import '../models/conversation.dart';
import '../models/message.dart';

class ConversationRepository {
  ConversationRepository(this._client);

  final ApiClient _client;

  Future<Conversation> create({String? title}) async {
    final res = await _client.post<Map<String, dynamic>>(
      '/v1/conversations',
      data: {'title': title},
    );
    return Conversation.fromJson(res.data!);
  }

  Future<List<Conversation>> list() async {
    final res = await _client.get<Map<String, dynamic>>('/v1/conversations');
    final list = res.data!['conversations'] as List;
    return list.map((e) => Conversation.fromJson(e as Map<String, dynamic>)).toList();
  }

  Future<Conversation> get(String id) async {
    final res = await _client.get<Map<String, dynamic>>('/v1/conversations/$id');
    return Conversation.fromJson(res.data!);
  }

  Future<List<Message>> listMessages(String conversationId) async {
    final res = await _client
        .get<Map<String, dynamic>>('/v1/conversations/$conversationId/messages');
    final list = res.data!['messages'] as List;
    return list.map((e) => Message.fromJson(e as Map<String, dynamic>)).toList();
  }

  /// Streams tutor response via SSE POST. Yields text deltas and completes with turn_completed.
  Stream<SseEnvelope> streamMessage({
    required String conversationId,
    required String content,
  }) async* {
    dev.log('ConversationRepository.streamMessage entered, convId: $conversationId, content: $content');
    final dio = _client.dio;
    final url = '${dio.options.baseUrl}/v1/conversations/$conversationId/messages';
    dev.log('ConversationRepository.streamMessage: POST to $url');
    final body = jsonEncode({'content': content});
    final headers = {
      'Accept': 'text/event-stream',
      'Content-Type': 'application/json',
      'Authorization': (_client.dio.options.headers['Authorization'] ?? '') as String,
    };

    final rawByteStream = sse_impl.sseStream(url, body, headers);
    dev.log('ConversationRepository.streamMessage: obtained rawByteStream');
    // Apply stateful utf8 decoder to safely handle multi-byte characters split across chunks!
    final rawStringStream = rawByteStream.transform(utf8.decoder);
    final lines = rawStringStream.transform(const LineSplitter());
    
    await for (final envelope in parseSseStream(lines)) {
      dev.log('ConversationRepository.streamMessage: yielding envelope ${envelope.event}');
      yield envelope;
    }
  }
}

final conversationRepositoryProvider = Provider<ConversationRepository>((ref) {
  final client = ref.watch(apiClientProvider);
  return ConversationRepository(client);
});
