use types::Query;
use super::config;
use types::QueriesSortType;
use std::collections::HashMap;
use web::wqq;
use std::sync::Mutex;
use std::thread::sleep;

lazy_static! {
    pub static ref qhash: Mutex<HashMap<String, usize>> = Mutex::new(HashMap::new());
}

fn sort(qq: &mut Vec<Query>) {
    let cnf = config.lock().unwrap();

    match cnf.sort_type {
        QueriesSortType::Timestamp =>
            qq.sort_by(|lhs, rhs|
                lhs.timestamp.partial_cmp(&rhs.timestamp).unwrap()),

        QueriesSortType::QueryTime =>
            qq.sort_by(|lhs, rhs|
                lhs.query_time.partial_cmp(&rhs.query_time).unwrap()),

        QueriesSortType::LockTime =>
            qq.sort_by(|lhs, rhs|
                lhs.lock_time.partial_cmp(&rhs.lock_time).unwrap()),

        QueriesSortType::RowsSent =>
            qq.sort_by(|lhs, rhs|
                lhs.rows_sent.partial_cmp(&rhs.rows_sent).unwrap()),

        QueriesSortType::RowsExamined =>
            qq.sort_by(|lhs, rhs|
                lhs.rows_examined.partial_cmp(&rhs.rows_examined).unwrap()),

        QueriesSortType::RowsAffected =>
            qq.sort_by(|lhs, rhs|
                lhs.rows_affected.partial_cmp(&rhs.rows_affected).unwrap()),

        QueriesSortType::TimestampInverse =>
            qq.sort_by(|lhs, rhs|
                rhs.timestamp.partial_cmp(&lhs.timestamp).unwrap()),

        QueriesSortType::QueryTimeInverse =>
            qq.sort_by(|lhs, rhs|
                rhs.query_time.partial_cmp(&lhs.query_time).unwrap()),

        QueriesSortType::LockTimeInverse =>
            qq.sort_by(|lhs, rhs|
                rhs.lock_time.partial_cmp(&lhs.lock_time).unwrap()),

        QueriesSortType::RowsSentInverse =>
            qq.sort_by(|lhs, rhs|
                rhs.rows_sent.partial_cmp(&lhs.rows_sent).unwrap()),

        QueriesSortType::RowsExaminedInverse =>
            qq.sort_by(|lhs, rhs|
                rhs.rows_examined.partial_cmp(&lhs.rows_examined).unwrap()),

        QueriesSortType::RowsAffectedInverse =>
            qq.sort_by(|lhs, rhs|
                rhs.rows_affected.partial_cmp(&lhs.rows_affected).unwrap()),

        _ => {}
    }
}

fn filter(qq: &Vec<Query>, mapflt: &mut usize) -> Vec<Query> {
    let cnf = config.lock().unwrap();

    qq.into_iter().filter(|q| {
        let not_filtered = q.timestamp >= cnf.timestamp_begin &&
            q.timestamp < cnf.timestamp_end &&
            q.query_time >= cnf.query_time_min &&
            q.query_time < cnf.query_time_max &&
            q.lock_time >= cnf.lock_time_min &&
            q.lock_time < cnf.lock_time_max &&
            q.rows_sent >= cnf.rows_sent_min &&
            q.rows_sent < cnf.rows_sent_max &&
            q.rows_examined >= cnf.rows_examined_min &&
            q.rows_examined < cnf.rows_examined_max &&
            q.rows_affected >= cnf.rows_affected_min &&
            q.rows_affected < cnf.rows_affected_max;

        if not_filtered {
            if let Some(regex) = &cnf.regex {
                let not_filter = !regex.find(&q.query).is_none();

                if !not_filter {
                    *mapflt += 1;
                }

                return not_filter;
            }
        }

        if !not_filtered {
            *mapflt += 1;
        }

        not_filtered
    }).cloned().collect()
}

fn make_dedup_hash(qq: &Vec<Query>) -> HashMap<&String, &Query> {
    let mut dedup_hash: HashMap<&String, &Query> = HashMap::new();

    for q in qq.iter() {
        if !dedup_hash.get(&q.query).is_none() {
            dedup_hash.remove(&q.query);
        }

        dedup_hash.insert(&q.query, q);
    }

    dedup_hash
}

pub fn process(qq: &mut Vec<Query>, web: bool) {
    let mut mapflt: usize = 0;
    let mut queries_hash = qhash.lock().unwrap();
    let wdelay = config.lock().unwrap().wpd;

    queries_hash.clear();

    for q in qq.iter() {
        let count = queries_hash.entry(q.query.clone()).or_insert(0);
        *count += 1;
        sleep(wdelay);
    }


    let mut new_qq = {
        if  config.lock().unwrap().dedup {
            let dedup_hash = make_dedup_hash(&qq);
            let mut dedupd_qq: Vec<Query> = Vec::new();

            for (_, &q) in dedup_hash.iter() {
                dedupd_qq.push(q.clone());
            }

            filter(&dedupd_qq, &mut mapflt)
        } else {
            filter(qq, &mut mapflt)
        }
    };

    sort(&mut new_qq);
    let cnf = config.lock().unwrap();

    {
        match cnf.sort_type {
            QueriesSortType::Count =>
                new_qq.sort_by(|lhs, rhs|
                    (*queries_hash.get(&lhs.query).unwrap())
                        .partial_cmp(queries_hash.get(&rhs.query).unwrap()).unwrap()),

            QueriesSortType::CountInverse =>
                new_qq.sort_by(|lhs, rhs|
                    (*queries_hash.get(&rhs.query).unwrap())
                        .partial_cmp(queries_hash.get(&lhs.query).unwrap()).unwrap()),

            _ => {}
        }
    }

    if web {
        let mut web_queries = wqq.lock().unwrap();

        web_queries.clear();
        web_queries.append(&mut new_qq);
    } else {
        for (index, q) in new_qq.iter().enumerate() {
            let count = queries_hash.get(&q.query).unwrap();

            if *count >= cnf.count_min && *count <= cnf.count_max {
                println!("{}", q.to_string(index + 1, *count));
            }

            if index == cnf.limit {
                break;
            }
        }

        println!("TOTAL: {}", qq.len());

        let filtered = (if new_qq.len() < cnf.limit { 0 } else { qq.len() - new_qq.len() }) +
            (if cnf.limit < new_qq.len() && (new_qq.len() - cnf.limit) > 0 { new_qq.len() - cnf.limit - 1 } else { 0 }) + mapflt;

        if filtered > 0 {
            println!("FILTERED: {}", filtered.to_string());
        }
    }

    qq.clear();
    new_qq.clear();
}
