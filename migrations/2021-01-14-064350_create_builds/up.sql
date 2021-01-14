CREATE TABLE `builds` (
  `build_id` int(11) NOT NULL,
  `project_id` int(11) NOT NULL,
  `build_number` int(11) NOT NULL,
  `branch` text NOT NULL,
  `files` text NOT NULL,
  `status` text NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

ALTER TABLE `builds`
  ADD PRIMARY KEY (`build_id`);