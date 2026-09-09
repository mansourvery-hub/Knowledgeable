import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../features/chat/chat_screen.dart';
import '../features/graph/graph_screen.dart';
import '../features/review/review_screen.dart';
import '../features/sessions/sessions_screen.dart';

final routerProvider = GoRouter(
  initialLocation: '/',
  routes: [
    GoRoute(
      path: '/',
      builder: (context, state) => const SessionsScreen(),
      routes: [
        GoRoute(
          path: 'c/:id',
          builder: (context, state) => ChatScreen(conversationId: state.pathParameters['id']!),
        ),
        GoRoute(
          path: 'graph',
          builder: (context, state) => const GraphScreen(),
        ),
        GoRoute(
          path: 'review',
          builder: (context, state) => const ReviewScreen(),
        ),
      ],
    ),
  ],
  errorBuilder: (context, state) => Scaffold(
    body: Center(child: Text(state.error.toString())),
  ),
);
