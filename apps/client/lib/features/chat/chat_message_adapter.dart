import 'package:flutter_chat_types/flutter_chat_types.dart' as types;
import '../../data/models/message.dart' as model;

class ChatMessageAdapter {
  static types.Message toFlyerMessage(model.Message msg, types.User user, types.User tutor) {
    return types.TextMessage(
      author: msg.isUser ? user : tutor,
      createdAt: msg.createdAt.millisecondsSinceEpoch,
      id: msg.id,
      text: msg.content,
    );
  }
}
