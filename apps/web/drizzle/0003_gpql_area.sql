CREATE SCHEMA IF NOT EXISTS "gpql";
--> statement-breakpoint
ALTER TABLE "public"."user" SET SCHEMA "gpql";
--> statement-breakpoint
ALTER TABLE "public"."session" SET SCHEMA "gpql";
--> statement-breakpoint
ALTER TABLE "public"."account" SET SCHEMA "gpql";
--> statement-breakpoint
ALTER TABLE "public"."verification" SET SCHEMA "gpql";
--> statement-breakpoint
ALTER TABLE "public"."sync_preference" SET SCHEMA "gpql";
--> statement-breakpoint
ALTER TABLE "public"."sync_recent" SET SCHEMA "gpql";
--> statement-breakpoint
ALTER TABLE "public"."sync_query" SET SCHEMA "gpql";
--> statement-breakpoint
ALTER TABLE "public"."erd_room" SET SCHEMA "gpql";
