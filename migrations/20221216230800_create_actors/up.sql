CREATE TABLE `actors` (
  `id` bigint(20) unsigned NOT NULL AUTO_INCREMENT,
  `playbook_id` bigint(20) unsigned NOT NULL,
  `name` varchar(255) NOT NULL,
  `description` varchar(512) NOT NULL,
  `repo` varchar(512) NOT NULL,
  `path` varchar(255) NOT NULL,
  `reference` varchar(255) NOT NULL,
  `commit` varchar(255) NOT NULL,
  `created_at` datetime DEFAULT CURRENT_TIMESTAMP,
  `updated_at` datetime DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB;
