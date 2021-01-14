CREATE TABLE `projects` (
  `project_id` int(11) NOT NULL,
  `project_name` text NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

ALTER TABLE `projects`
  ADD PRIMARY KEY (`project_id`);