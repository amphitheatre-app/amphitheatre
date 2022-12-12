ALTER TABLE `playbooks` ADD `created_at` DATETIME  NULL  DEFAULT now();
ALTER TABLE `playbooks` ADD `updated_at` DATETIME  NULL  DEFAULT now()  AFTER `created_at`;
