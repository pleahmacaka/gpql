CREATE TABLE `preference` (
	`key` text PRIMARY KEY NOT NULL,
	`value` text NOT NULL
);
--> statement-breakpoint
CREATE TABLE `recent` (
	`url` text PRIMARY KEY NOT NULL,
	`kind` text NOT NULL,
	`label` text NOT NULL,
	`detail` text NOT NULL,
	`opened_at` integer DEFAULT (unixepoch()) NOT NULL
);
--> statement-breakpoint
CREATE INDEX `recent_opened_at` ON `recent` (`opened_at`);--> statement-breakpoint
CREATE TABLE `saved_query` (
	`id` text PRIMARY KEY NOT NULL,
	`name` text NOT NULL,
	`sql` text NOT NULL,
	`target` text DEFAULT '' NOT NULL,
	`saved_at` integer DEFAULT (unixepoch()) NOT NULL
);
