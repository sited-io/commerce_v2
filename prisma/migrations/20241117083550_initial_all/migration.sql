-- CreateEnum
CREATE TYPE "OfferTypeKey" AS ENUM ('PHYSICAL', 'DIGITAL');

-- CreateEnum
CREATE TYPE "PriceTypeKey" AS ENUM ('ONE_TIME', 'RECURRING');

-- CreateEnum
CREATE TYPE "OrderTypeKey" AS ENUM ('ONE_OFF', 'SUBSCRIPTION');

-- CreateEnum
CREATE TYPE "PaymentMethodKey" AS ENUM ('STRIPE');

-- CreateEnum
CREATE TYPE "StripeAccountStatus" AS ENUM ('PENDING', 'CONFIGURED');

-- CreateTable
CREATE TABLE "offers" (
    "offer_id" UUID NOT NULL,
    "owner" TEXT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL,
    "offer_type" "OfferTypeKey" NOT NULL,

    CONSTRAINT "offers_pkey" PRIMARY KEY ("offer_id")
);

-- CreateTable
CREATE TABLE "offer_details" (
    "offer_id" UUID NOT NULL,
    "owner" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "description" TEXT,

    CONSTRAINT "offer_details_pkey" PRIMARY KEY ("offer_id")
);

-- CreateTable
CREATE TABLE "offer_prices" (
    "offer_id" UUID NOT NULL,
    "owner" TEXT NOT NULL,
    "unit_amount" INTEGER NOT NULL,
    "currency" TEXT NOT NULL,
    "price_type" "PriceTypeKey" NOT NULL,

    CONSTRAINT "offer_prices_pkey" PRIMARY KEY ("offer_id")
);

-- CreateTable
CREATE TABLE "price_recurring" (
    "offer_id" UUID NOT NULL,
    "interval" TEXT NOT NULL,
    "interval_count" INTEGER NOT NULL,
    "trial_period_days" INTEGER,

    CONSTRAINT "price_recurring_pkey" PRIMARY KEY ("offer_id")
);

-- CreateTable
CREATE TABLE "offer_shipping_rates" (
    "offer_id" UUID NOT NULL,
    "owner" TEXT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL,
    "unit_amount" INTEGER NOT NULL,
    "currency" TEXT NOT NULL,
    "all_countries" BOOLEAN NOT NULL,
    "specific_countries" TEXT[],

    CONSTRAINT "offer_shipping_rates_pkey" PRIMARY KEY ("offer_id")
);

-- CreateTable
CREATE TABLE "offer_images" (
    "offer_image_id" UUID NOT NULL,
    "offer_id" UUID NOT NULL,
    "owner" TEXT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL,
    "image_url" TEXT NOT NULL,
    "ordering" INTEGER NOT NULL,

    CONSTRAINT "offer_images_pkey" PRIMARY KEY ("offer_image_id")
);

-- CreateTable
CREATE TABLE "offer_files" (
    "offer_file_id" UUID NOT NULL,
    "offer_id" UUID NOT NULL,
    "owner" TEXT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL,
    "file_name" TEXT NOT NULL,
    "content_type" TEXT,
    "total_size_bytes" BIGINT NOT NULL,
    "uploaded_size_bytes" BIGINT NOT NULL,
    "file_path" TEXT NOT NULL,
    "file_url" TEXT NOT NULL,
    "ordering" INTEGER NOT NULL,

    CONSTRAINT "offer_files_pkey" PRIMARY KEY ("offer_file_id")
);

-- CreateTable
CREATE TABLE "user_quotas" (
    "user_id" UUID NOT NULL,
    "max_allowed_size_bytes" BIGINT NOT NULL,
    "uploaded_size_bytes" BIGINT NOT NULL DEFAULT 0,

    CONSTRAINT "user_quotas_pkey" PRIMARY KEY ("user_id")
);

-- CreateTable
CREATE TABLE "shops" (
    "shop_id" UUID NOT NULL,
    "owner" TEXT NOT NULL,
    "website_id" TEXT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL,

    CONSTRAINT "shops_pkey" PRIMARY KEY ("shop_id")
);

-- CreateTable
CREATE TABLE "orders" (
    "order_id" UUID NOT NULL,
    "buyer_user_id" TEXT NOT NULL,
    "offer_id" UUID NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL,
    "order_type" "OrderTypeKey" NOT NULL,
    "payment_method" "PaymentMethodKey" NOT NULL,

    CONSTRAINT "orders_pkey" PRIMARY KEY ("order_id")
);

-- CreateTable
CREATE TABLE "order_type_one_off" (
    "order_id" UUID NOT NULL,
    "payed_at" TIMESTAMPTZ,

    CONSTRAINT "order_type_one_off_pkey" PRIMARY KEY ("order_id")
);

-- CreateTable
CREATE TABLE "order_type_subscription" (
    "order_id" UUID NOT NULL,
    "current_period_start" TIMESTAMPTZ NOT NULL,
    "current_period_end" TIMESTAMPTZ NOT NULL,
    "status" TEXT NOT NULL,
    "payed_at" TIMESTAMPTZ,
    "payed_untill" TIMESTAMPTZ,
    "cancelled_at" TIMESTAMPTZ,
    "cancel_at" TIMESTAMPTZ,

    CONSTRAINT "order_type_subscription_pkey" PRIMARY KEY ("order_id")
);

-- CreateTable
CREATE TABLE "payment_method_stripe" (
    "order_id" UUID NOT NULL,
    "stripe_subscription_id" TEXT,

    CONSTRAINT "payment_method_stripe_pkey" PRIMARY KEY ("order_id")
);

-- CreateTable
CREATE TABLE "stripe_accounts" (
    "stripe_account_id" TEXT NOT NULL,
    "website_id" TEXT NOT NULL,
    "owner" TEXT NOT NULL,
    "status" "StripeAccountStatus" NOT NULL,

    CONSTRAINT "stripe_accounts_pkey" PRIMARY KEY ("stripe_account_id")
);

-- CreateTable
CREATE TABLE "stripe_account_status_pending" (
    "stripe_account_id" TEXT NOT NULL,
    "link" TEXT NOT NULL,

    CONSTRAINT "stripe_account_status_pending_pkey" PRIMARY KEY ("stripe_account_id")
);

-- CreateTable
CREATE TABLE "stripe_account_status_configured" (
    "stripe_account_id" TEXT NOT NULL,
    "charges_enabled" BOOLEAN NOT NULL,
    "details_submitted" BOOLEAN NOT NULL,

    CONSTRAINT "stripe_account_status_configured_pkey" PRIMARY KEY ("stripe_account_id")
);

-- CreateTable
CREATE TABLE "sub_websites" (
    "website_id" TEXT NOT NULL,
    "owner" TEXT NOT NULL,

    CONSTRAINT "sub_websites_pkey" PRIMARY KEY ("website_id")
);

-- CreateTable
CREATE TABLE "_OfferToShop" (
    "A" UUID NOT NULL,
    "B" UUID NOT NULL
);

-- CreateIndex
CREATE UNIQUE INDEX "stripe_accounts_website_id_key" ON "stripe_accounts"("website_id");

-- CreateIndex
CREATE UNIQUE INDEX "_OfferToShop_AB_unique" ON "_OfferToShop"("A", "B");

-- CreateIndex
CREATE INDEX "_OfferToShop_B_index" ON "_OfferToShop"("B");

-- AddForeignKey
ALTER TABLE "offer_details" ADD CONSTRAINT "offer_details_offer_id_fkey" FOREIGN KEY ("offer_id") REFERENCES "offers"("offer_id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "offer_prices" ADD CONSTRAINT "offer_prices_offer_id_fkey" FOREIGN KEY ("offer_id") REFERENCES "offers"("offer_id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "price_recurring" ADD CONSTRAINT "price_recurring_offer_id_fkey" FOREIGN KEY ("offer_id") REFERENCES "offer_prices"("offer_id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "offer_shipping_rates" ADD CONSTRAINT "offer_shipping_rates_offer_id_fkey" FOREIGN KEY ("offer_id") REFERENCES "offers"("offer_id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "offer_images" ADD CONSTRAINT "offer_images_offer_id_fkey" FOREIGN KEY ("offer_id") REFERENCES "offers"("offer_id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "offer_files" ADD CONSTRAINT "offer_files_offer_id_fkey" FOREIGN KEY ("offer_id") REFERENCES "offers"("offer_id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "order_type_one_off" ADD CONSTRAINT "order_type_one_off_order_id_fkey" FOREIGN KEY ("order_id") REFERENCES "orders"("order_id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "order_type_subscription" ADD CONSTRAINT "order_type_subscription_order_id_fkey" FOREIGN KEY ("order_id") REFERENCES "orders"("order_id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "payment_method_stripe" ADD CONSTRAINT "payment_method_stripe_order_id_fkey" FOREIGN KEY ("order_id") REFERENCES "orders"("order_id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "stripe_account_status_pending" ADD CONSTRAINT "stripe_account_status_pending_stripe_account_id_fkey" FOREIGN KEY ("stripe_account_id") REFERENCES "stripe_accounts"("stripe_account_id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "stripe_account_status_configured" ADD CONSTRAINT "stripe_account_status_configured_stripe_account_id_fkey" FOREIGN KEY ("stripe_account_id") REFERENCES "stripe_accounts"("stripe_account_id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "_OfferToShop" ADD CONSTRAINT "_OfferToShop_A_fkey" FOREIGN KEY ("A") REFERENCES "offers"("offer_id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "_OfferToShop" ADD CONSTRAINT "_OfferToShop_B_fkey" FOREIGN KEY ("B") REFERENCES "shops"("shop_id") ON DELETE CASCADE ON UPDATE CASCADE;
