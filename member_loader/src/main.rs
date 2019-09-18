use xml::reader::{EventReader, XmlEvent};
use chrono::{DateTime, Utc};
use serde::{Serialize};

const MEMBER_API_BASE: &str = "https://data.parliament.uk/membersdataplatform/services/mnisv1.0/Members/Query/";

#[derive(Debug, Clone)]
enum House {
    Commons,
    Lords,
    Unknown,
}

impl Into<&str> for House {
    fn into(self) -> &'static str {
        match self {
            House::Commons => "Commons",
            House::Lords => "Lords",
            House::Unknown => unimplemented!()
        }
    }
}

enum AdditionalData {
    Addresses,
    Parties,
}

impl Into<&str> for AdditionalData {
    fn into(self) -> &'static str {
        match self {
            AdditionalData::Addresses => "Addresses",
            AdditionalData::Parties => "Parties",
        }
    }
}

#[derive(Debug)]
struct Members {
    members: Vec<Member>
}

impl Members {
    fn new() -> Self {
        Members {
            members: vec![]
        }
    }
}

#[derive(Debug, Clone)]
struct Member {
    id: u32,
    name: String,
    party: String,
    house: House,
    constituency: String,
    twitter: Option<String>,
    facebook: Option<String>,
    parties: Vec<Party>,
}

impl Member {
    fn new() -> Self {
        Self {
            id: 0,
            name: "".to_string(),
            party: "".to_string(),
            house: House::Unknown,
            constituency: "".to_string(),
            twitter: None,
            facebook: None,
            parties: vec![],
        }
    }
}


#[derive(Debug, Clone)]
struct Party {
    name: String,
    start_date: DateTime<Utc>,
    end_date: Option<DateTime<Utc>>,
}

impl Party {
    fn new() -> Self {
        Self {
            name: "".to_string(),
            start_date: Utc::now(),
            end_date: None,
        }
    }
}

fn get_api_url(house: House, additional_data: Vec<AdditionalData>) -> String {
    let mut url = format!("{}house={}|isEligible=true/", MEMBER_API_BASE, Into::<&str>::into(house));

    let data: Vec<&str> = additional_data.into_iter().map(|d| Into::<&str>::into(d)).collect();
    url.extend(format!("{}/", data.join("|")).chars());

    url
}

fn merge_parties(parties: &Vec<Party>) -> Vec<Party> {
    let mut parties = parties.clone();
    if parties.len() < 2 {
        return parties;
    }

    let mut out: Vec<Party> = vec![];
    loop {
        let a = parties.pop();
        let b = parties.pop();
        match (a, b) {
            (Some(a), Some(b)) => {
                if a.name == b.name {
                    parties.push(Party {
                        name: a.name,
                        start_date: b.start_date,
                        end_date: a.end_date,
                    });
                } else {
                    out.push(a);
                    parties.push(b);
                }
            }
            (Some(a), None) => {
                out.push(a);
                return out;
            }
            (None, None) => return out,
            (None, Some(_)) => unreachable!()
        }
    }
}

fn parse_addresses_xml(data: &str) -> Option<Members> {
    let parser = EventReader::from_str(data);

    #[derive(Debug, PartialEq)]
    enum Element {
        None,
        Members,
        Member,
        Addresses,
        Address,
        AddrType,
        AddressLine1,
        Parties,
        Party,
        PartyPartyName,
        PartyStartDate,
        PartyEndDate,
        Name,
        PartyName,
        House,
        Constituency,
        Other,
    }

    #[derive(Debug)]
    struct Address {
        addr_type: String,
        address: String,
    }

    impl Address {
        fn new() -> Self {
            Self {
                addr_type: "".to_string(),
                address: "".to_string(),
            }
        }
    }

    let mut members: Option<Members> = None;
    let mut member: Option<Member> = None;
    let mut address: Option<Address> = None;
    let mut party: Option<Party> = None;

    let mut current_element = Element::None;
    let mut previous_elements: Vec<Element> = vec![];

    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                if current_element == Element::None {
                    if name.local_name == "Members" {
                        members = Some(Members::new());
                        previous_elements.push(current_element);
                        current_element = Element::Members;
                        continue;
                    }
                } else if current_element == Element::Members {
                    if name.local_name == "Member" {
                        let mut id = None;
                        for a in attributes {
                            if a.name.local_name == "Member_Id" {
                                id = Some(a.value.parse::<u32>().unwrap())
                            }
                        }

                        let mut m = Member::new();
                        m.id = id.unwrap();
                        member = Some(m);
                        previous_elements.push(current_element);
                        current_element = Element::Member;
                        continue;
                    }
                } else if current_element == Element::Member {
                    if name.local_name == "Addresses" {
                        previous_elements.push(current_element);
                        current_element = Element::Addresses;
                        continue;
                    } else if name.local_name == "Parties" {
                        previous_elements.push(current_element);
                        current_element = Element::Parties;
                        continue;
                    } else if name.local_name == "DisplayAs" {
                        previous_elements.push(current_element);
                        current_element = Element::Name;
                        continue;
                    } else if name.local_name == "Party" {
                        previous_elements.push(current_element);
                        current_element = Element::PartyName;
                        continue;
                    } else if name.local_name == "House" {
                        previous_elements.push(current_element);
                        current_element = Element::House;
                        continue;
                    } else if name.local_name == "MemberFrom" {
                        previous_elements.push(current_element);
                        current_element = Element::Constituency;
                        continue;
                    }
                } else if current_element == Element::Addresses {
                    if name.local_name == "Address" {
                        address = Some(Address::new());
                        previous_elements.push(current_element);
                        current_element = Element::Address;
                        continue;
                    }
                } else if current_element == Element::Address {
                    if name.local_name == "Type" {
                        previous_elements.push(current_element);
                        current_element = Element::AddrType;
                        continue;
                    } else if name.local_name == "Address1" {
                        previous_elements.push(current_element);
                        current_element = Element::AddressLine1;
                        continue;
                    }
                } else if current_element == Element::Parties {
                    if name.local_name == "Party" {
                        party = Some(Party::new());
                        previous_elements.push(current_element);
                        current_element = Element::Party;
                        continue;
                    }
                } else if current_element == Element::Party {
                    if name.local_name == "Name" {
                        previous_elements.push(current_element);
                        current_element = Element::PartyPartyName;
                        continue;
                    } else if name.local_name == "StartDate" {
                        previous_elements.push(current_element);
                        current_element = Element::PartyStartDate;
                        continue;
                    } else if name.local_name == "EndDate" {
                        for a in attributes {
                            if a.name.local_name == "nil" && a.value == "true" {
                                continue;
                            }
                        }

                        previous_elements.push(current_element);
                        current_element = Element::PartyEndDate;
                        continue;
                    }
                }
                previous_elements.push(current_element);
                current_element = Element::Other;
            }
            Ok(XmlEvent::EndElement { name }) => {
                if name.local_name == "Member" {
                    match &mut members {
                        Some(members) => members.members.push(member.clone().unwrap().clone()),
                        None => unreachable!()
                    }
                } else if name.local_name == "Address" {
                    match &mut member {
                        None => unreachable!(),
                        Some(member) => {
                            match &address {
                                Some(address) => match address.addr_type.as_str() {
                                    "Twitter" => member.twitter = Some(address.address.clone()),
                                    "Facebook" => member.facebook = Some(address.address.clone()),
                                    _ => {}
                                },
                                None => unreachable!()
                            }
                        }
                    }
                } else if name.local_name == "Party" && current_element == Element::Party {
                    match &mut member {
                        None => unreachable!(),
                        Some(member) => {
                            match &party {
                                Some(party) => member.parties.push(party.clone()),
                                None => unreachable!()
                            }
                        }
                    }
                } else if name.local_name == "Parties" && current_element == Element::Parties {
                    match &mut member {
                        None => unreachable!(),
                        Some(member) => {
                            member.parties.sort_by(|a, b| a.start_date.timestamp().partial_cmp(&b.start_date.timestamp()).unwrap());
                            member.parties = merge_parties(&member.parties);
                        }
                    }
                }
                current_element = match previous_elements.pop() {
                    Some(e) => e,
                    None => Element::None
                };
            }
            Ok(XmlEvent::Characters(data)) => {
                match current_element {
                    Element::None | Element::Other | Element::Members | Element::Member |
                    Element::Addresses | Element::Address |
                    Element::Parties | Element::Party => {}
                    Element::Name => {
                        match &mut member {
                            None => unreachable!(),
                            Some(member) => member.name = data.clone()
                        }
                    }
                    Element::PartyName => {
                        match &mut member {
                            None => unreachable!(),
                            Some(member) => member.party = data.clone()
                        }
                    }
                    Element::Constituency => {
                        match &mut member {
                            None => unreachable!(),
                            Some(member) => member.constituency = data.clone()
                        }
                    }
                    Element::House => {
                        match &mut member {
                            None => unreachable!(),
                            Some(member) => member.house = match data.as_str() {
                                "Commons" => House::Commons,
                                "Lords" => House::Lords,
                                _ => House::Unknown
                            }
                        }
                    }
                    Element::AddrType => {
                        match &mut address {
                            None => unreachable!(),
                            Some(address) => address.addr_type = data.clone()
                        }
                    }
                    Element::AddressLine1 => {
                        match &mut address {
                            None => unreachable!(),
                            Some(address) => address.address = data.clone()
                        }
                    }
                    Element::PartyPartyName => {
                        match &mut party {
                            None => unreachable!(),
                            Some(party) => party.name = data.clone()
                        }
                    }
                    Element::PartyStartDate => {
                        match &mut party {
                            None => unreachable!(),
                            Some(party) => party.start_date = match format!("{}Z", data.clone()).parse::<DateTime<Utc>>() {
                                Ok(d) => d,
                                Err(_) => panic!("{}", data),
                            }
                        }
                    }
                    Element::PartyEndDate => {
                        match &mut party {
                            None => unreachable!(),
                            Some(party) => party.end_date = Some(match format!("{}Z", data.clone()).parse::<DateTime<Utc>>() {
                                Ok(d) => d,
                                Err(_) => panic!("{}", data),
                            })
                        }
                    }
                }
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
            _ => {}
        }
    }

    members
}

fn commit_member_data(dgraph: dgraph::Dgraph, members: Members) {
    let mut txn = dgraph.new_txn();

    #[derive(Serialize, Debug)]
    struct MemberObject {
        uid: String,
        name: String,
    }

    for member in members.members {
        let m = MemberObject {
            uid: format!("_:{}", member.id),
            name: member.name,
        };

        let mb = serde_json::to_vec(&m).expect("Invalid json");

        let mu = dgraph::Mutation {
            set_json: mb,
            ..Default::default()
        };

        txn.mutate(mu).expect("failed to create member");
    }

    txn.commit().expect("Failed to commit txn");
}

fn main() {
    println!("Connecting to dgraph...");
    let dgraph = dgraph::make_dgraph!(dgraph::new_dgraph_client("localhost:9080"));

    println!("Getting House of Commons data...");
    let commons_addresses_url = get_api_url(House::Commons, vec![AdditionalData::Addresses, AdditionalData::Parties]);
    let commons_addresses_body = reqwest::get(commons_addresses_url.as_str()).unwrap().text().unwrap();

    println!("Parsing House of Commons data...");
    let commons_members = parse_addresses_xml(&commons_addresses_body).unwrap();
    println!("{:#?}", commons_members);

    println!("Commiting data...");
    commit_member_data(dgraph, commons_members);
}
