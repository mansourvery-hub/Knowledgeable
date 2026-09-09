import 'package:flutter/material.dart';

class GraphScreen extends StatelessWidget {
  const GraphScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Knowledge Graph')),
      body: const Center(child: Text('Graph inspection — secondary UI (Phase 6)')),
    );
  }
}
