-- AlterTable
ALTER TABLE "orders" ALTER COLUMN "buyer_user_id" DROP NOT NULL;

-- AddForeignKey
ALTER TABLE "orders" ADD CONSTRAINT "orders_offer_id_fkey" FOREIGN KEY ("offer_id") REFERENCES "offers"("offer_id") ON DELETE RESTRICT ON UPDATE CASCADE;
