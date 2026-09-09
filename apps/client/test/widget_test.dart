import 'package:flutter_test/flutter_test.dart';
import 'package:knowledgeable_client/app/app.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

void main() {
  testWidgets('App renders', (WidgetTester tester) async {
    await tester.pumpWidget(const ProviderScope(child: KnowledgeableApp()));
    expect(find.text('Knowledgeable'), findsOneWidget);
  });
}
