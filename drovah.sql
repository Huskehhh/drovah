CREATE TABLE `builds` (
  `project_id` int(11) NOT NULL,
  `build_number` int(11) DEFAULT NULL,
  `branch` text DEFAULT NULL,
  `files` text DEFAULT NULL,
  `build_timestamp` datetime DEFAULT current_timestamp(),
  `status` text NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

CREATE TABLE `projects` (
  `project_id` int(11) NOT NULL,
  `project_name` text NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

ALTER TABLE `projects`
  ADD PRIMARY KEY (`project_id`);