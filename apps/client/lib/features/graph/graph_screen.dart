import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'graph_provider.dart';

class GraphScreen extends ConsumerWidget {
  const GraphScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final graphAsync = ref.watch(graphDataProvider);

    return Scaffold(
      appBar: AppBar(title: const Text('Knowledge Graph (Debug)')),
      body: graphAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, st) => Center(child: Text('Error: $e')),
        data: (data) {
          final nodes = data['nodes'] as List<dynamic>? ?? [];
          final links = data['links'] as List<dynamic>? ?? [];
          
          return Column(
            children: [
              Expanded(
                flex: 2,
                child: ListView.builder(
                  itemCount: nodes.length,
                  itemBuilder: (context, index) {
                    final node = nodes[index] as Map<String, dynamic>;
                    return ListTile(
                      title: Text(node['canonical_name'] ?? 'Unnamed'),
                      subtitle: Text(node['canonical_statement'] ?? ''),
                    );
                  },
                ),
              ),
              const Divider(),
              Expanded(
                flex: 1,
                child: ListView.builder(
                  itemCount: links.length,
                  itemBuilder: (context, index) {
                    final link = links[index] as Map<String, dynamic>;
                    return ListTile(
                      title: Text('Relation: ${link['relation_type']}'),
                      subtitle: Text('From: ${link['from_concept_id']} -> To: ${link['to_concept_id']}'),
                    );
                  },
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}
