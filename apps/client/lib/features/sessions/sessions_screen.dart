import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

class SessionsScreen extends ConsumerWidget {
  const SessionsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Scaffold(
      appBar: AppBar(title: const Text('Knowledgeable')),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const Text('Ask something to learn from your frontier'),
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: () => context.go('/c/new'),
              child: const Text('New conversation'),
            ),
            const SizedBox(height: 8),
            TextButton(
              onPressed: () => context.go('/graph'),
              child: const Text('Graph (secondary)'),
            ),
            TextButton(
              onPressed: () => context.go('/review'),
              child: const Text('Review'),
            ),
          ],
        ),
      ),
    );
  }
}
