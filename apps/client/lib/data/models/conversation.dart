class Conversation {
  const Conversation({
    required this.id,
    required this.learnerId,
    this.title,
    required this.createdAt,
    required this.updatedAt,
  });

  factory Conversation.fromJson(Map<String, dynamic> json) => Conversation(
        id: json['id'] as String,
        learnerId: json['learner_id'] as String,
        title: json['title'] as String?,
        createdAt: DateTime.parse(json['created_at'] as String),
        updatedAt: DateTime.parse(json['updated_at'] as String),
      );

  final String id;
  final String learnerId;
  final String? title;
  final DateTime createdAt;
  final DateTime updatedAt;
}
