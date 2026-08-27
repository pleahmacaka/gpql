CREATE TABLE `chat_log` (
	`id` text PRIMARY KEY NOT NULL,
	`title` text DEFAULT '' NOT NULL,
	`turns` text NOT NULL,
	`saved_at` integer DEFAULT (unixepoch()) NOT NULL
);
