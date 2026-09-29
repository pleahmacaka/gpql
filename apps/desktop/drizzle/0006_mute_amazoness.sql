CREATE TABLE `sync_tombstone` (
	`kind` text NOT NULL,
	`key` text NOT NULL,
	`deleted_at` integer NOT NULL,
	PRIMARY KEY(`kind`, `key`)
);
--> statement-breakpoint
ALTER TABLE `preference` ADD `updated_at` integer DEFAULT 0 NOT NULL;