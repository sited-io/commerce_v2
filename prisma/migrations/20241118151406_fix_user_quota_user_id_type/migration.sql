/*
  Warnings:

  - The primary key for the `user_quotas` table will be changed. If it partially fails, the table could be left without primary key constraint.

*/
-- AlterTable
ALTER TABLE "user_quotas" DROP CONSTRAINT "user_quotas_pkey",
ALTER COLUMN "user_id" SET DATA TYPE TEXT,
ADD CONSTRAINT "user_quotas_pkey" PRIMARY KEY ("user_id");
