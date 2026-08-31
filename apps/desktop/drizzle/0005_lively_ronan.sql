ALTER TABLE `recent` ADD `folder` text;--> statement-breakpoint
ALTER TABLE `recent` ADD `rank` integer DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE `recent` ADD `tunnelled` integer DEFAULT 0 NOT NULL;