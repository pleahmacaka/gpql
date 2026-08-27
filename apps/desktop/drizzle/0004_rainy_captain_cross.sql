CREATE TABLE `query_run` (
	`id` text PRIMARY KEY NOT NULL,
	`sql` text NOT NULL,
	`target` text DEFAULT '' NOT NULL,
	`ok` integer DEFAULT true NOT NULL,
	`millis` integer DEFAULT 0 NOT NULL,
	`ran_at` integer DEFAULT (unixepoch()) NOT NULL
);
--> statement-breakpoint
CREATE INDEX `query_run_ran_at` ON `query_run` (`ran_at`);