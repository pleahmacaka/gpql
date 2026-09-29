ALTER TABLE "gpql"."account" ADD COLUMN IF NOT EXISTS "issuer" text;--> statement-breakpoint
ALTER TABLE "gpql"."account" ALTER COLUMN "issuer" DROP NOT NULL;--> statement-breakpoint
UPDATE "gpql"."account" SET "issuer" = CASE WHEN "provider_id" = 'credential' THEN 'local:credential' ELSE 'local:oauth:' || "provider_id" END WHERE "issuer" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "account_issuer_idx" ON "gpql"."account" USING btree ("issuer","account_id");--> statement-breakpoint
ALTER TABLE "gpql"."sync_query" DROP CONSTRAINT "sync_query_pkey";--> statement-breakpoint
ALTER TABLE "gpql"."sync_query" ADD CONSTRAINT "sync_query_user_id_id_pk" PRIMARY KEY("user_id","id");--> statement-breakpoint
ALTER TABLE "gpql"."sync_query" ADD COLUMN "deleted_at" integer;--> statement-breakpoint
ALTER TABLE "gpql"."sync_preference" ADD COLUMN "updated_at" integer DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "gpql"."sync_recent" ADD COLUMN "deleted_at" integer;--> statement-breakpoint
ALTER TABLE "gpql"."erd_room" ADD COLUMN "layout" text;--> statement-breakpoint
ALTER TABLE "gpql"."erd_room" ADD COLUMN "version" integer DEFAULT 0 NOT NULL;--> statement-breakpoint
CREATE INDEX "erd_room_user_id_idx" ON "gpql"."erd_room" USING btree ("user_id");
