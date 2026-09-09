import 'package:drift/drift.dart';
import 'package:drift_flutter/drift_flutter.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Drift + SQLite local cache — server is authoritative.
/// Per architecture: SQLite is disposable cache, never competing source of truth.
/// Phase 0: minimal database, no generated tables yet — full schema via build_runner in Phase 7.

class Conversations extends Table {
  TextColumn get id => text()();
  TextColumn get title => text().nullable()();
  DateTimeColumn get createdAt => dateTime()();
  DateTimeColumn get updatedAt => dateTime()();

  @override
  Set<Column> get primaryKey => {id};
}

class ConceptCache extends Table {
  TextColumn get id => text()();
  TextColumn get canonicalName => text()();
  TextColumn get canonicalStatement => text()();
  TextColumn get learnerStatement => text().nullable()();
  RealColumn get worldConfidence => real()();
  RealColumn get learnerConfidence => real().nullable()();
  TextColumn get status => text().withDefault(const Constant('active'))();
  DateTimeColumn get updatedAt => dateTime()();

  @override
  Set<Column> get primaryKey => {id};
}

// Minimal database without codegen — replaces @DriftDatabase for Phase 0
// Full codegen will be restored in Phase 7 (Sync, Offline Cache)
// ignore: prefer_void_to_null
class AppDatabase extends GeneratedDatabase {
  AppDatabase() : super(driftDatabase(name: 'knowledgeable'));

  AppDatabase.forTesting(super.executor);

  @override
  int get schemaVersion => 1;

  @override
  List<TableInfo<Table, Object?>> get allTables => [];

  @override
  List<DatabaseSchemaEntity> get allSchemaEntities => [];
}

final appDatabaseProvider = Provider<AppDatabase>((ref) {
  final db = AppDatabase();
  ref.onDispose(() => db.close());
  return db;
});
